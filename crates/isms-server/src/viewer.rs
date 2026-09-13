//! Visibility (TDD 10.3): the engine records everything; the API decides who
//! sees what, once, here. Your own true output versus others' attributed
//! figures, managers' per-worker views, and the private ledger events.

use isms_core::event::Event;
use isms_core::ids::{CitizenId, OrgId, WorkplaceId};
use isms_core::plan::touches;
use isms_core::rules::Rules;
use isms_core::world::World;
use serde_json::Value;
use std::collections::BTreeSet;

/// Event kinds only their parties (and the managers of an org named in them) may read.
const PRIVATE_KINDS: &[&str] = &[
    "Paid",
    "PaymentMissed",
    "DividendPaid",
    "CreditInstallment",
    "CreditMissed",
    "CreditRepaid",
    "CreditDefaulted",
    "RentPaid",
    "RentMissed",
    "Transferred",
    "CitizenSeen",
    "PlanChanged",
    "LaborSet",
    "Drew",
    "StoreDrawRequested",
    "StoreReturned",
];

#[derive(Clone, Debug)]
pub struct Viewer {
    pub citizen: Option<CitizenId>,
    /// Orgs this citizen manages.
    pub manages: BTreeSet<OrgId>,
    /// With sigma = 0 the attributed figure equals the true one; nothing to hide.
    pub sigma_zero: bool,
}

impl Viewer {
    #[must_use]
    pub fn new(world: &World, citizen: Option<CitizenId>) -> Self {
        let manages = citizen.map_or_else(BTreeSet::new, |c| {
            world
                .orgs
                .values()
                .filter(|o| o.manager == Some(c))
                .map(|o| o.id)
                .collect()
        });
        let sigma = Rules::from_world(world).capabilities.monitoring_sigma;
        Viewer {
            citizen,
            manages,
            sigma_zero: sigma == 0.0,
        }
    }

    /// A spectator: public events only.
    #[must_use]
    pub fn public(world: &World) -> Self {
        Viewer::new(world, None)
    }

    #[must_use]
    pub fn is_self(&self, c: CitizenId) -> bool {
        self.citizen == Some(c)
    }

    #[must_use]
    pub fn manages_workplace(&self, world: &World, wp: WorkplaceId) -> bool {
        world
            .workplaces
            .get(&wp)
            .is_some_and(|w| self.manages.contains(&w.org))
    }

    /// May this viewer see per-worker figures for `citizen` at `wp`?
    /// Returns `(attributed, true_output)` visibility.
    #[must_use]
    pub fn worker_figures(
        &self,
        world: &World,
        wp: WorkplaceId,
        citizen: CitizenId,
    ) -> (bool, bool) {
        if self.is_self(citizen) {
            (true, true)
        } else if self.manages_workplace(world, wp) {
            (true, self.sigma_zero)
        } else {
            (false, false)
        }
    }

    /// The event as this viewer may see it; `None` when hidden entirely.
    #[must_use]
    pub fn view_event(&self, world: &World, event: &Event) -> Option<Value> {
        let kind = event.kind();
        let mut value = serde_json::to_value(event).ok()?;
        match event {
            Event::Produced {
                workplace,
                per_worker,
                ..
            } => {
                let mut kept = Vec::with_capacity(per_worker.len());
                for w in per_worker {
                    let (attributed, true_output) =
                        self.worker_figures(world, *workplace, w.citizen);
                    if !attributed {
                        continue;
                    }
                    let mut wv = serde_json::to_value(w).ok()?;
                    if !true_output {
                        wv["true_output"] = Value::Null;
                        // The Explain names the true figure; the manager gets the rule, not the number.
                        if let Some(explain) = wv.get_mut("explain") {
                            explain["result"] = Value::Null;
                        }
                    }
                    kept.push(wv);
                }
                value["Produced"]["per_worker"] = Value::Array(kept);
                Some(value)
            }
            Event::TickResolved {
                citizen_deltas,
                workplace_deltas,
                ..
            } => {
                let mine: Vec<Value> = citizen_deltas
                    .iter()
                    .filter(|d| self.is_self(d.citizen))
                    .filter_map(|d| serde_json::to_value(d).ok())
                    .collect();
                let managed: Vec<Value> = workplace_deltas
                    .iter()
                    .filter(|d| self.manages_workplace(world, d.workplace))
                    .filter_map(|d| serde_json::to_value(d).ok())
                    .collect();
                value["TickResolved"]["citizen_deltas"] = Value::Array(mine);
                value["TickResolved"]["workplace_deltas"] = Value::Array(managed);
                Some(value)
            }
            _ if PRIVATE_KINDS.contains(&kind) => {
                let mine = self.citizen.is_some_and(|c| touches(event, c));
                let org = value
                    .get(kind)
                    .and_then(|p| p.get("org"))
                    .and_then(Value::as_u64)
                    .and_then(|n| u32::try_from(n).ok())
                    .map(OrgId);
                let managed = org.is_some_and(|o| self.manages.contains(&o));
                (mine || managed).then_some(value)
            }
            _ => Some(value),
        }
    }
}

/// Every `Explain` object inside a (viewer-filtered) event payload.
#[must_use]
pub fn explains_in(value: &Value) -> Vec<Value> {
    fn walk(v: &Value, out: &mut Vec<Value>) {
        match v {
            Value::Object(map) => {
                if map.contains_key("rule")
                    && map.contains_key("formula")
                    && map.contains_key("inputs")
                {
                    out.push(v.clone());
                    return;
                }
                for child in map.values() {
                    walk(child, out);
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(value, &mut out);
    out
}
