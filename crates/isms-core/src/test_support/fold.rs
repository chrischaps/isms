//! Fold a log into a `World` and compare it with a live one.

use crate::apply::apply;
use crate::event::Event;
use crate::world::World;

/// Fold a complete log (starting with `SocietyCreated`) into a fresh `World`.
///
/// # Panics
/// If the log does not start with `SocietyCreated`.
#[must_use]
pub fn fold(events: &[Event]) -> World {
    let Some(Event::SocietyCreated {
        society_id,
        seed,
        preset,
    }) = events.first()
    else {
        panic!("fold: the log must start with SocietyCreated");
    };
    let mut world = World::new(*society_id, *seed, preset);
    for e in events {
        apply(&mut world, e);
    }
    world
}

/// Assert that folding `log` reproduces `live` byte for byte, printing a field
/// diff of the JSON forms on mismatch.
pub fn assert_fold_equals_live(log: &[Event], live: &World) {
    let folded = fold(log);
    if folded.hash() == live.hash() {
        return;
    }
    let a = serde_json::to_value(&folded).unwrap();
    let b = serde_json::to_value(live).unwrap();
    let mut diffs = Vec::new();
    diff_json("", &a, &b, &mut diffs);
    diffs.truncate(20);
    panic!(
        "fold != live after {} events; first differences (folded vs live):\n{}",
        log.len(),
        diffs.join("\n")
    );
}

fn diff_json(path: &str, a: &serde_json::Value, b: &serde_json::Value, out: &mut Vec<String>) {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(ma), Value::Object(mb)) => {
            let keys: std::collections::BTreeSet<&String> = ma.keys().chain(mb.keys()).collect();
            for k in keys {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                match (ma.get(k), mb.get(k)) {
                    (Some(x), Some(y)) => diff_json(&p, x, y, out),
                    (Some(x), None) => out.push(format!("{p}: {x} vs <absent>")),
                    (None, Some(y)) => out.push(format!("{p}: <absent> vs {y}")),
                    (None, None) => {}
                }
            }
        }
        (Value::Array(xa), Value::Array(xb)) if xa.len() == xb.len() => {
            for (i, (x, y)) in xa.iter().zip(xb).enumerate() {
                diff_json(&format!("{path}[{i}]"), x, y, out);
            }
        }
        _ if a != b => out.push(format!("{path}: {a} vs {b}")),
        _ => {}
    }
}
