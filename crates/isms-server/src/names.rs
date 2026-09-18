//! Names for the ids in an engine refusal (S1.15, D2). The engine speaks in
//! ids (`c41 already works at w19`) so its texts are stable and its tests
//! plain; a person, the CLI and a synthetic player should read names. The web
//! did this on the way in (`web/src/lib/names.ts`); the server now does it for
//! every client, from the same two directories: citizens and orgs.
//!
//! Tokens are a letter and a number (`isms-core` ids.rs): `c` citizen, `o` org,
//! `w` workplace, `d` dwelling, `k` contract, `f` offer, `r` order, `s` slot.

use isms_core::command::Reject;
use isms_core::ids::CitizenId;
use isms_core::world::World;
use std::collections::BTreeMap;

/// What a refusal may name, keyed by id.
pub struct Directory {
    /// Handles by citizen id.
    pub citizens: BTreeMap<u32, String>,
    /// Org names by org id.
    pub orgs: BTreeMap<u32, String>,
    /// "farm at Greenfield", or "machine shop 2 at Ironworks" when the org has
    /// several of a kind, by workplace id.
    pub workplaces: BTreeMap<u32, String>,
    /// The reader, who is "you".
    pub me: Option<u32>,
}

impl Directory {
    /// The directories a refusal to `me` is read against.
    #[must_use]
    pub fn from_world(world: &World, me: Option<CitizenId>) -> Self {
        let citizens = world
            .citizens
            .values()
            .map(|c| (c.id.0, c.handle.clone()))
            .collect();
        let orgs: BTreeMap<u32, String> = world
            .orgs
            .values()
            .map(|o| (o.id.0, o.name.clone()))
            .collect();
        let mut workplaces = BTreeMap::new();
        for o in world.orgs.values() {
            // In id order; numbered only when the org has several of a kind.
            let mut wps: Vec<_> = o
                .workplaces
                .iter()
                .filter_map(|w| world.workplaces.get(w))
                .collect();
            wps.sort_by_key(|w| w.id);
            for w in &wps {
                let kind = kind_name(w.kind);
                let same: Vec<_> = wps.iter().filter(|x| x.kind == w.kind).collect();
                let title = if same.len() > 1 {
                    let n = same.iter().position(|x| x.id == w.id).unwrap_or(0) + 1;
                    format!("{kind} {n}")
                } else {
                    kind
                };
                workplaces.insert(w.id.0, format!("{title} at {}", o.name));
            }
        }
        Directory {
            citizens,
            orgs,
            workplaces,
            me: me.map(|c| c.0),
        }
    }

    fn citizen(&self, n: u32) -> String {
        if self.me == Some(n) {
            return "you".into();
        }
        self.citizens
            .get(&n)
            .cloned()
            .unwrap_or_else(|| format!("citizen no. {n}"))
    }

    fn org(&self, n: u32) -> String {
        self.orgs
            .get(&n)
            .cloned()
            .unwrap_or_else(|| format!("organization no. {n}"))
    }

    fn token(&self, letter: char, n: u32) -> String {
        match letter {
            'c' => self.citizen(n),
            'o' => self.org(n),
            'w' => format!(
                "the {}",
                self.workplaces
                    .get(&n)
                    .cloned()
                    .unwrap_or_else(|| format!("workplace no. {n}"))
            ),
            'd' => format!("dwelling no. {n}"),
            's' => format!("slot {n}"),
            'k' => "that contract".into(),
            'f' => "that offer".into(),
            _ => "that order".into(),
        }
    }

    /// The refusal with its ids named, its nouns and verbs agreed, and the
    /// engine's cycles and ticks called days and hours.
    #[must_use]
    pub fn reject(&self, r: Reject) -> Reject {
        Reject::new(r.code, self.text(&r.message))
    }

    /// `c41 already works at w19` -> `You already work at the mill at Hollow Mill`.
    #[must_use]
    pub fn text(&self, text: &str) -> String {
        let unwrapped = unwrap_debug(text);
        // A handle or an org's name keeps its own spelling at the start of a
        // sentence, whether the engine wrote the id or a pass already named it.
        let first = first_word(&unwrapped);
        let starts_with_name = match parse_token(first) {
            Some(('c', n)) => self.me != Some(n),
            Some(('o', _)) => true,
            _ => {
                self.citizens.values().any(|h| h == first)
                    || self
                        .orgs
                        .values()
                        .any(|o| unwrapped.starts_with(o.as_str()))
            }
        };
        let words: Vec<&str> = unwrapped.split(' ').collect();
        let mut out: Vec<String> = Vec::with_capacity(words.len());
        let mut i = 0;
        while i < words.len() {
            let w = words[i];
            let (core, tail) = split_punct(w);
            // "no workplace w19": the id names nothing, so say so.
            if core == "no"
                && i + 2 < words.len()
                && NOUNS.contains(&words[i + 1])
                && parse_token(split_punct(words[i + 2]).0).is_some()
            {
                let noun = if words[i + 1] == "org" {
                    "organization"
                } else {
                    words[i + 1]
                };
                let (_, tail2) = split_punct(words[i + 2]);
                out.push(format!("there is no such {noun}{tail2}"));
                i += 3;
                continue;
            }
            // "contract k441 is ...": the token alone carries the noun.
            if NOUNS.contains(&core)
                && tail.is_empty()
                && i + 1 < words.len()
                && parse_token(split_punct(words[i + 1]).0).is_some()
            {
                i += 1;
                continue;
            }
            if let Some((letter, n)) = parse_token(core) {
                let named = self.token(letter, n);
                let you = named == "you";
                out.push(format!("{named}{tail}"));
                i += 1;
                // "you already works" -> "you already work".
                if you && i < words.len() {
                    let mut j = i;
                    if words[j] == "already" {
                        out.push("already".into());
                        j += 1;
                    }
                    if j < words.len() {
                        let (verb, vtail) = split_punct(words[j]);
                        if let Some(plural) = agree(verb) {
                            out.push(format!("{plural}{vtail}"));
                            i = j + 1;
                            continue;
                        }
                    }
                    if j > i {
                        i = j;
                    }
                }
                continue;
            }
            out.push(units(core, tail));
            i += 1;
        }
        let joined = out.join(" ");
        if starts_with_name {
            joined
        } else {
            capitalize(&joined)
        }
    }
}

const NOUNS: [&str; 8] = [
    "citizen",
    "org",
    "workplace",
    "dwelling",
    "contract",
    "offer",
    "order",
    "slot",
];

/// `works` -> `work` after "you"; None for a word that is not such a verb.
fn agree(verb: &str) -> Option<&'static str> {
    Some(match verb {
        "works" => "work",
        "holds" => "hold",
        "has" => "have",
        "does" => "do",
        "is" => "are",
        "owns" => "own",
        "manages" => "manage",
        "controls" => "control",
        _ => return None,
    })
}

/// cycles -> days, ticks -> hours; anything else as it was.
fn units(word: &str, tail: &str) -> String {
    let w = match word {
        "cycle" => "day",
        "cycles" => "days",
        "cycle's" => "day's",
        "tick" => "hour",
        "ticks" => "hours",
        other => other,
    };
    format!("{w}{tail}")
}

fn kind_name(kind: isms_core::kinds::WorkplaceKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.replace('_', " ")))
        .unwrap_or_default()
}

/// `Citizen(c41)` and `Org(o15)`, the `{:?}` of a `Party`, to their tokens.
fn unwrap_debug(text: &str) -> String {
    let mut s = text.to_owned();
    for wrapper in ["Citizen(", "Org("] {
        while let Some(start) = s.find(wrapper) {
            let inner = start + wrapper.len();
            let Some(close) = s[inner..].find(')') else {
                break;
            };
            let token = s[inner..inner + close].to_owned();
            if parse_token(&token).is_some() {
                s.replace_range(start..inner + close + 1, &token);
            } else {
                break;
            }
        }
    }
    s
}

/// A letter and a number, the engine's id form; nothing else.
fn parse_token(word: &str) -> Option<(char, u32)> {
    let mut chars = word.chars();
    let letter = chars.next()?;
    if !"cowdkfrs".contains(letter) {
        return None;
    }
    let digits = chars.as_str();
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((letter, digits.parse().ok()?))
}

/// A word and the punctuation that trails it.
fn split_punct(word: &str) -> (&str, &str) {
    let end = word.trim_end_matches(['.', ',', ';', ':', ')']).len();
    word.split_at(end)
}

fn first_word(text: &str) -> &str {
    split_punct(text.split(' ').next().unwrap_or("")).0
}

fn capitalize(text: &str) -> String {
    let mut c = text.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(me: Option<u32>) -> Directory {
        Directory {
            citizens: [(41, "chris".to_owned()), (5, "otto".to_owned())].into(),
            orgs: [
                (19, "Hollow Mill".to_owned()),
                (2, "Legacy Farm No. 1".to_owned()),
            ]
            .into(),
            workplaces: [
                (19, "mill at Hollow Mill".to_owned()),
                (2, "farm at Legacy Farm No. 1".to_owned()),
            ]
            .into(),
            me,
        }
    }

    #[test]
    fn names_the_citizen_and_the_workplace_and_agrees_the_verb_with_you() {
        let mine = dir(Some(41));
        let theirs = dir(Some(5));
        assert_eq!(
            mine.text("c41 already works at w19"),
            "You already work at the mill at Hollow Mill"
        );
        assert_eq!(
            theirs.text("c41 already works at w19"),
            "chris already works at the mill at Hollow Mill"
        );
        assert_eq!(
            mine.text("c41 already holds a position elsewhere"),
            "You already hold a position elsewhere"
        );
        assert_eq!(
            mine.text("c41 does not control Legacy Farm No. 1"),
            "You do not control Legacy Farm No. 1"
        );
        assert_eq!(
            mine.text("c41 has no position at w2"),
            "You have no position at the farm at Legacy Farm No. 1"
        );
    }

    #[test]
    fn drops_a_noun_the_token_carries_and_unwraps_a_debug_printed_party() {
        let mine = dir(Some(41));
        assert_eq!(
            mine.text("contract k441 is not an open employment"),
            "That contract is not an open employment"
        );
        assert_eq!(
            mine.text("offer f30 is addressed to Citizen(c5)"),
            "That offer is addressed to otto"
        );
        assert_eq!(
            mine.text("Citizen(c5) holds 3 shares of o19"),
            "otto holds 3 shares of Hollow Mill"
        );
        assert_eq!(mine.text("Citizen(c41) has 965.94"), "You have 965.94");
        assert_eq!(mine.text("Org(o19) has 0.00"), "Hollow Mill has 0.00");
        assert_eq!(
            mine.text("Citizen(c41) does not own d1"),
            "You do not own dwelling no. 1"
        );
    }

    #[test]
    fn says_a_missing_thing_is_missing_instead_of_naming_its_number() {
        let mine = dir(Some(41));
        assert_eq!(mine.text("no workplace w77"), "There is no such workplace");
        assert_eq!(mine.text("no org o3"), "There is no such organization");
        assert_eq!(
            mine.text("no dwelling d999999"),
            "There is no such dwelling"
        );
        assert_eq!(mine.text("no order r1"), "There is no such order");
        assert_eq!(mine.text("no offer f81"), "There is no such offer");
        assert_eq!(
            mine.text("offer f81 was taken or withdrawn"),
            "That offer was taken or withdrawn"
        );
    }

    #[test]
    fn speaks_of_days_and_hours_and_leaves_ordinary_words_alone() {
        let mine = dir(Some(41));
        assert_eq!(
            mine.text("a loan runs 1..=12 cycles"),
            "A loan runs 1..=12 days"
        );
        assert_eq!(
            mine.text("exceeds this cycle's budget of 8 h"),
            "Exceeds this day's budget of 8 h"
        );
        assert_eq!(
            mine.text("the contract allows at most 8 h at w19"),
            "The contract allows at most 8 h at the mill at Hollow Mill"
        );
        assert_eq!(
            mine.text("a destitute citizen cannot sign a long contract"),
            "A destitute citizen cannot sign a long contract"
        );
        assert_eq!(
            mine.text("cannot transfer to yourself"),
            "Cannot transfer to yourself"
        );
    }

    #[test]
    fn an_unknown_id_is_spelled_out_and_a_named_text_is_left_alone() {
        let mine = dir(None);
        assert_eq!(
            mine.text("c7 does not manage o19"),
            "citizen no. 7 does not manage Hollow Mill"
        );
        let named = mine.text("c41 already works at w19");
        assert_eq!(mine.text(&named), named, "naming is idempotent");
    }
}
