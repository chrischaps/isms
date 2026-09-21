//! The Chronicle projection (TDD 9.4, GDD 12): a pure `Projector` turns the
//! event stream into headlines using the preset's templates, and a task feeds
//! it from the actor's broadcast. Everything it needs it learns from the
//! events themselves (names from joins and foundings, the previous cycle's
//! index from the previous `CycleClosed`), so `rebuild-projections` replays
//! the log through the same code and gets the same rows.

use crate::actor::SocietyHandle;
use isms_core::event::Event;
use isms_store::projections::NewHeadline;
use isms_store::{EventStore, PgEventStore, StoreError};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

fn hundred() -> f64 {
    100.0
}

#[derive(Clone, Debug, Deserialize)]
pub struct Thresholds {
    pub price_move_pct: f64,
    /// The night's verdict where a `need_met` line exists (S2.9): at or above
    /// this fulfilment `need_met` fires, below it `need_short`.
    #[serde(default = "hundred")]
    pub need_met_pct: f64,
}

/// `presets/copy/<preset>/chronicle.toml`.
#[derive(Clone, Debug, Deserialize)]
pub struct Templates {
    pub thresholds: Thresholds,
    /// Event kind -> template, or `Kind.path=value` -> a variant that fires
    /// when the payload field at `path` reads `value` (S2.9).
    pub templates: BTreeMap<String, String>,
    /// `price_up`, `price_down`, `hardship`, `stock_out`, `need_met`, `need_short`.
    pub cycle: BTreeMap<String, String>,
    /// Enum values in the society's words, `[words.<field>] value = "..."`;
    /// the Welcome Brief reads them (S2.9).
    #[serde(default)]
    pub words: BTreeMap<String, BTreeMap<String, String>>,
}

impl Templates {
    pub fn load(presets_dir: &Path, preset: &str) -> Result<Templates, String> {
        let path = presets_dir.join("copy").join(preset).join("chronicle.toml");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("chronicle copy {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("chronicle copy {}: {e}", path.display()))
    }

    /// `value` in the society's words, or `value` itself where the copy has none.
    #[must_use]
    pub fn word<'a>(&'a self, field: &str, value: &'a str) -> &'a str {
        self.words
            .get(field)
            .and_then(|t| t.get(value))
            .map_or(value, String::as_str)
    }
}

/// Everything a society needs from `presets/copy/<preset>/` to be served:
/// the Chronicle templates and the Welcome Brief. `seed` calls this before
/// it writes a row, so a preset without copy fails there and not at the
/// first tick or the first visitor (S2.9).
pub fn check_copy(presets_dir: &Path, preset: &str) -> Result<(), String> {
    // `presets/test/*` are engine fixtures (TDD S1.4), never served: no copy.
    if preset.starts_with("test/") {
        return Ok(());
    }
    Templates::load(presets_dir, preset)?;
    let welcome = presets_dir.join("copy").join(preset).join("welcome.md");
    std::fs::metadata(&welcome).map_err(|e| format!("welcome copy {}: {e}", welcome.display()))?;
    Ok(())
}

/// Names learned from the log, so ids read as people and firms.
#[derive(Clone, Debug, Default)]
pub struct Names {
    pub citizens: BTreeMap<u64, String>,
    pub orgs: BTreeMap<u64, String>,
    /// Workplace id -> its kind, from `WorkplaceAdded` and `WorkplaceOpened`.
    pub workplaces: BTreeMap<u64, String>,
    /// Proposal id -> its title, from `Proposed`; an admission vote is named
    /// after the citizen and the org.
    pub proposals: BTreeMap<u64, String>,
    /// Citizens who are people; householder doings are scenery, not news.
    pub humans: std::collections::BTreeSet<u64>,
}

/// Templates whose `citizen` must be a human to make a headline.
const ABOUT_A_PERSON: &[&str] = &[
    "ManagerAppointed",
    "HardshipBegan",
    "HardshipEnded",
    "DestitutionBegan",
    "DestitutionEnded",
];

/// Pure headline generator.
#[derive(Clone, Debug)]
pub struct Projector {
    templates: Templates,
    names: Names,
    prev_index: Option<f64>,
    /// The policy in force, as JSON, from `SocietyCreated` and every
    /// `PolicyChanged` since: the next change is narrated field by field
    /// against it (S2.9).
    policy: Option<Value>,
}

fn field<'a>(payload: &'a Value, path: &str) -> Option<&'a Value> {
    let mut v = payload;
    for part in path.split('.') {
        v = match (v, part.parse::<usize>()) {
            (Value::Array(items), Ok(i)) => items.get(i)?,
            _ => v.get(part)?,
        };
    }
    Some(v)
}

/// A variant key `Kind.path=value` (S2.9): the path and the value it wants.
fn variant_of<'a>(key: &'a str, kind: &str) -> Option<(&'a str, &'a str)> {
    let rest = key.strip_prefix(kind)?.strip_prefix('.')?;
    rest.split_once('=')
}

fn plain(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.as_f64().map_or_else(
            || n.to_string(),
            |f| {
                if f.fract() == 0.0 {
                    format!("{f:.0}")
                } else {
                    format!("{f:.2}")
                }
            },
        )),
        Value::Bool(b) => Some(b.to_string()),
        other => Some(other.to_string()),
    }
}

impl Projector {
    #[must_use]
    pub fn new(templates: Templates) -> Self {
        Projector {
            templates,
            names: Names::default(),
            prev_index: None,
            policy: None,
        }
    }

    fn learn(&mut self, event: &Event) {
        match event {
            Event::SocietyCreated { preset, .. } => {
                self.policy = serde_json::to_value(&preset.policy).ok();
            }
            Event::WorkplaceAdded {
                workplace, kind, ..
            }
            | Event::WorkplaceOpened {
                workplace, kind, ..
            } => {
                let kind = serde_json::to_value(kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_else(|| format!("{kind:?}").to_lowercase());
                self.names
                    .workplaces
                    .insert(u64::from(workplace.0), kind.replace('_', " "));
            }
            Event::Proposed {
                proposal, title, ..
            } => {
                self.names
                    .proposals
                    .insert(u64::from(proposal.0), title.clone());
            }
            Event::AdmissionProposed {
                proposal,
                org,
                citizen,
                ..
            } => {
                let who = self.name_citizen(u64::from(citizen.0));
                let org = self.name_org(u64::from(org.0));
                self.names.proposals.insert(
                    u64::from(proposal.0),
                    format!("the admission of {who} to {org}"),
                );
            }
            Event::CitizenJoined {
                citizen,
                handle,
                kind,
                ..
            } => {
                self.names
                    .citizens
                    .insert(u64::from(citizen.0), handle.clone());
                if *kind == isms_core::kinds::CitizenKind::Human {
                    self.names.humans.insert(u64::from(citizen.0));
                }
            }
            Event::HouseholderJoined {
                citizen, handle, ..
            } => {
                self.names
                    .citizens
                    .insert(u64::from(citizen.0), handle.clone());
            }
            Event::OrgFounded { org, name, .. } => {
                self.names.orgs.insert(u64::from(org.0), name.clone());
            }
            _ => {}
        }
    }

    /// Fill `{{...}}` placeholders; `None` when any referenced field is missing.
    fn fill(&self, template: &str, payload: &Value) -> Option<String> {
        let mut out = String::with_capacity(template.len());
        let mut rest = template;
        while let Some(start) = rest.find("{{") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let end = after.find("}}")?;
            let spec = &after[..end];
            let (kind, path) = spec.split_once(':').unwrap_or(("", spec));
            let value = field(payload, path)?;
            let text = match kind {
                "" => plain(value)?,
                "citizen" => self.name_citizen(value.as_u64()?),
                "org" => self.name_org(value.as_u64()?),
                "workplace" => {
                    let id = value.as_u64()?;
                    self.names
                        .workplaces
                        .get(&id)
                        .map_or_else(|| format!("workplace #{id}"), |k| format!("the {k} #{id}"))
                }
                "proposal" => {
                    let id = value.as_u64()?;
                    self.names
                        .proposals
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| format!("proposal #{id}"))
                }
                "credits" => {
                    let cents = value.as_i64()?;
                    format!("{}.{:02}", cents / 100, (cents % 100).abs())
                }
                "pct" => format!("{:.0}", value.as_f64()? * 100.0),
                _ => return None,
            };
            out.push_str(&text);
            rest = &after[end + 2..];
        }
        out.push_str(rest);
        Some(out)
    }

    fn name_citizen(&self, id: u64) -> String {
        self.names
            .citizens
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("citizen #{id}"))
    }

    fn name_org(&self, id: u64) -> String {
        self.names
            .orgs
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("org #{id}"))
    }

    /// Every template for `kind` that applies to `payload`: the plain one,
    /// then each `Kind.path=value` variant whose field reads `value`, in key
    /// order (S2.9).
    fn render_kind(&self, kind: &str, payload: &Value, out: &mut Vec<String>) {
        if let Some(t) = self.templates.templates.get(kind)
            && let Some(text) = self.fill(t, payload)
        {
            out.push(text);
        }
        let prefix = format!("{kind}.");
        for (key, t) in self.templates.templates.range(prefix.clone()..) {
            if !key.starts_with(&prefix) {
                break;
            }
            let Some((path, want)) = variant_of(key, kind) else {
                continue;
            };
            if field(payload, path)
                .and_then(plain)
                .is_some_and(|v| v == want)
                && let Some(text) = self.fill(t, payload)
            {
                out.push(text);
            }
        }
    }

    /// One line per policy field that moved (S2.9): `PolicyChanged.<field>=<to>`
    /// where the copy names the value, else `PolicyChanged.<field>` with
    /// `{{from}}` and `{{to}}`. Nothing before the first `SocietyCreated`.
    fn render_policy_change(&self, next: &Value, out: &mut Vec<String>) {
        let (Some(Value::Object(prev)), Value::Object(next)) = (&self.policy, next) else {
            return;
        };
        for (name, to) in next {
            let from = prev.get(name).unwrap_or(&Value::Null);
            if from == to {
                continue;
            }
            let ctx = serde_json::json!({ "field": name, "from": from, "to": to });
            let keyed = plain(to).map(|v| format!("PolicyChanged.{name}={v}"));
            let template = keyed
                .as_ref()
                .and_then(|k| self.templates.templates.get(k))
                .or_else(|| {
                    self.templates
                        .templates
                        .get(&format!("PolicyChanged.{name}"))
                });
            if let Some(t) = template
                && let Some(text) = self.fill(t, &ctx)
            {
                out.push(text);
            }
        }
    }

    /// Headlines for one event, in order.
    pub fn observe(&mut self, event: &Event, seq: i64, tick: u32, cycle: u32) -> Vec<NewHeadline> {
        self.learn(event);
        let kind = event.kind();
        let mut payload = serde_json::to_value(event)
            .ok()
            .and_then(|v| v.get(kind).cloned())
            .unwrap_or(Value::Null);
        let mut texts: Vec<String> = Vec::new();
        match event {
            // The copy keys on the outcome, which the tally implies (S2.9).
            Event::ProposalClosed { passed, tally, .. } => {
                let outcome = if *passed {
                    "passed"
                } else if tally.cast < tally.quorum {
                    "no_quorum"
                } else {
                    "failed"
                };
                if let Value::Object(map) = &mut payload {
                    map.insert("outcome".into(), Value::String(outcome.into()));
                }
            }
            Event::PolicyChanged { policy, .. } => {
                let next = serde_json::to_value(policy).unwrap_or(Value::Null);
                self.render_policy_change(&next, &mut texts);
                self.policy = Some(next);
            }
            _ => {}
        }
        let about_a_householder = ABOUT_A_PERSON.contains(&kind)
            && !payload
                .get("citizen")
                .and_then(Value::as_u64)
                .is_some_and(|c| self.names.humans.contains(&c));
        if !about_a_householder && !matches!(event, Event::PolicyChanged { .. }) {
            self.render_kind(kind, &payload, &mut texts);
        }
        if let Event::CycleClosed { aggregates, .. } = event {
            if let (Some(prev), Some(now)) = (self.prev_index, aggregates.price_index) {
                let pct = (now / prev - 1.0) * 100.0;
                let key = if pct >= 0.0 { "price_up" } else { "price_down" };
                if pct.abs() >= self.templates.thresholds.price_move_pct
                    && let Some(t) = self.templates.cycle.get(key)
                {
                    let ctx = serde_json::json!({ "pct": format!("{:.0}", pct.abs()), "index": format!("{now:.2}") });
                    if let Some(text) = self.fill(t, &ctx) {
                        texts.push(text);
                    }
                }
            }
            if aggregates.price_index.is_some() {
                self.prev_index = aggregates.price_index;
            }
            if aggregates.hardship_count > 0
                && let Some(t) = self.templates.cycle.get("hardship")
            {
                let ctx = serde_json::json!({ "count": aggregates.hardship_count });
                if let Some(text) = self.fill(t, &ctx) {
                    texts.push(text);
                }
            }
            // A bare shelf at the close of the day (GDD 12 "stock-outs"; S2.9).
            if let Some(t) = self.templates.cycle.get("stock_out") {
                for (good, qty) in &aggregates.store_stock {
                    if *qty > 0 {
                        continue;
                    }
                    let name = serde_json::to_value(good)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_else(|| format!("{good:?}").to_lowercase());
                    let ctx = serde_json::json!({ "good": name });
                    if let Some(text) = self.fill(t, &ctx) {
                        texts.push(text);
                    }
                }
            }
            // The night's verdict on need (GDD 6.2 scoreboard; S2.9).
            let pct = aggregates.need_fulfillment_rate * 100.0;
            let key = if pct >= self.templates.thresholds.need_met_pct {
                "need_met"
            } else {
                "need_short"
            };
            if let Some(t) = self.templates.cycle.get(key) {
                let ctx = serde_json::json!({ "pct": format!("{pct:.0}") });
                if let Some(text) = self.fill(t, &ctx) {
                    texts.push(text);
                }
            }
        }
        let tick = i32::try_from(tick).unwrap_or(i32::MAX);
        let cycle = i32::try_from(cycle).unwrap_or(i32::MAX);
        texts
            .into_iter()
            .enumerate()
            .map(|(i, headline)| NewHeadline {
                source_event_seq: seq,
                ordinal: i32::try_from(i).unwrap_or(i32::MAX),
                cycle,
                tick,
                headline,
            })
            .collect()
    }
}

/// Replay the whole log through a fresh projector and insert what it yields
/// (idempotent). Returns the projector, caught up, and the last seq seen.
pub async fn replay(
    store: &PgEventStore,
    society: i64,
    templates: Templates,
) -> Result<(Projector, i64), StoreError> {
    let mut projector = Projector::new(templates);
    let mut after = -1i64;
    loop {
        let page = store.read_from(society, after, 5_000).await?;
        let Some(last) = page.last() else { break };
        after = last.seq;
        let mut rows = Vec::new();
        for e in &page {
            rows.extend(projector.observe(&e.event, e.seq, e.meta.tick, e.meta.cycle));
        }
        store.insert_headlines(society, &rows).await?;
    }
    Ok((projector, after))
}

/// Clear and regenerate one society's Chronicle (`isms-server rebuild-projections`).
pub async fn rebuild(
    store: &PgEventStore,
    society: i64,
    templates: Templates,
) -> Result<usize, StoreError> {
    store.clear_chronicle(society).await?;
    replay(store, society, templates).await?;
    Ok(store.all_headlines(society).await?.len())
}

/// Catch up from the log, then follow the actor's broadcast until cancelled.
pub fn spawn_projector(
    store: PgEventStore,
    handle: SocietyHandle,
    templates: Templates,
    cancel: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let society = handle.id;
        // Subscribe first so nothing committed during the catch-up is missed;
        // inserts are idempotent, so overlap is harmless.
        let mut rx = handle.subscribe();
        let (mut projector, mut last_seq) = match replay(&store, society, templates).await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(society, "chronicle catch-up failed: {e}");
                return;
            }
        };
        loop {
            tokio::select! {
                () = cancel.cancelled() => return,
                batch = rx.recv() => match batch {
                    Ok(b) => {
                        let mut rows = Vec::new();
                        for (i, e) in b.events.iter().enumerate() {
                            let seq = b.first_seq + i64::try_from(i).unwrap_or(0);
                            if seq <= last_seq {
                                continue;
                            }
                            rows.extend(projector.observe(e, seq, b.tick, b.cycle));
                            last_seq = seq;
                        }
                        if let Err(e) = store.insert_headlines(society, &rows).await {
                            tracing::error!(society, "chronicle insert failed: {e}");
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(society, lagged = n, "chronicle projector fell behind; replaying");
                        match replay(&store, society, projector_templates(&projector)).await {
                            Ok((p, s)) => {
                                projector = p;
                                last_seq = s;
                            }
                            Err(e) => tracing::error!(society, "chronicle replay failed: {e}"),
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    })
}

fn projector_templates(p: &Projector) -> Templates {
    p.templates.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use isms_core::event::CycleAggregates;
    use isms_core::ids::{CitizenId, OrgId};
    use isms_core::kinds::{CitizenKind, OrgKind};
    use isms_core::money::Money;
    use isms_core::world::Ownership;

    fn templates() -> Templates {
        Templates::load(Path::new(isms_core::WORKSPACE_PRESETS_DIR), "freeport").unwrap()
    }

    fn aggregates(index: Option<f64>, hardship: u32) -> CycleAggregates {
        CycleAggregates {
            population: 41,
            active_humans: 1,
            householders: 40,
            real_output: 0.0,
            median_wellbeing: 90.0,
            need_fulfillment_rate: 1.0,
            consumption_gini: 0.0,
            investment_share: 0.0,
            price_index: index,
            mean_cycle_wage: 0.0,
            unemployed: 0,
            firm_count: 18,
            credit_outstanding: Money(0),
            hardship_count: hardship,
            store_stock: BTreeMap::new(),
            low_population_cycles: 0,
            ..Default::default()
        }
    }

    /// The S1.5 gate: a founding, a 12 % price move, and a hardship give three headlines.
    #[test]
    fn founding_price_move_and_hardship_make_three_headlines() {
        let mut p = Projector::new(templates());
        let joined = Event::CitizenJoined {
            citizen: CitizenId(41),
            handle: "marlow".into(),
            kind: CitizenKind::Human,
            endowment: Money(100_000),
            dwelling: None,
            explain: None,
        };
        let mut all = p.observe(&joined, 1, 0, 0);
        let founded = Event::OrgFounded {
            org: OrgId(18),
            kind: OrgKind::Firm,
            name: "Iron & Sons".into(),
            founder: Some(CitizenId(41)),
            ownership: Ownership::Shares {
                issued: 100,
                holdings: BTreeMap::new(),
            },
            manager: Some(CitizenId(41)),
            fee_burned: Money(0),
        };
        all.extend(p.observe(&founded, 2, 5, 0));
        let c0 = Event::CycleClosed {
            cycle: 0,
            aggregates: aggregates(Some(1.0), 0),
            low_population_cycles: 0,
            reached_floor: false,
        };
        all.extend(p.observe(&c0, 3, 23, 0));
        let hardship = Event::HardshipBegan {
            citizen: CitizenId(41),
            cycle: 1,
        };
        all.extend(p.observe(&hardship, 4, 47, 1));
        let c1 = Event::CycleClosed {
            cycle: 1,
            aggregates: aggregates(Some(1.12), 1),
            low_population_cycles: 0,
            reached_floor: false,
        };
        all.extend(p.observe(&c1, 5, 47, 1));
        let texts: Vec<&str> = all.iter().map(|h| h.headline.as_str()).collect();
        assert!(
            texts.iter().any(|t| t.starts_with("marlow arrives")),
            "{texts:?}"
        );
        assert!(
            texts.contains(&"marlow founds Iron & Sons, a new firm."),
            "{texts:?}"
        );
        assert!(
            texts.contains(&"marlow can no longer keep food on the table."),
            "{texts:?}"
        );
        assert!(
            texts.contains(
                &"Prices rose 12% over the cycle. The basket costs 1.12 of what it did on day one."
            ),
            "{texts:?}"
        );
        assert!(texts.contains(&"1 went hungry this cycle."), "{texts:?}");
        // A 5 % move is below the threshold: no price headline at cycle 2.
        let c2 = Event::CycleClosed {
            cycle: 2,
            aggregates: aggregates(Some(1.176), 0),
            low_population_cycles: 0,
            reached_floor: false,
        };
        assert!(p.observe(&c2, 6, 71, 2).is_empty());
    }
}
