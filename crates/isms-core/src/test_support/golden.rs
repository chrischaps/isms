//! Golden event logs: byte-compared postcard, with a `.jsonl` twin so a diff is
//! readable in review. `UPDATE_GOLDEN=1` rewrites.

use crate::event::Event;
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"))
}

/// Compare `events` with `tests/golden/<name>.postcard`. A missing golden is
/// written and accepted (commit it); a mismatch fails unless `UPDATE_GOLDEN=1`.
pub fn check_golden(name: &str, events: &[Event]) {
    let dir = golden_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let bin_path = dir.join(format!("{name}.postcard"));
    let jsonl_path = dir.join(format!("{name}.jsonl"));
    let bytes = postcard::to_allocvec(events).unwrap();
    let jsonl: String = events
        .iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let update = std::env::var("UPDATE_GOLDEN").is_ok_and(|v| v == "1");
    match std::fs::read(&bin_path) {
        Ok(existing) if existing == bytes => {}
        Ok(existing) if update => {
            std::fs::write(&bin_path, &bytes).unwrap();
            std::fs::write(&jsonl_path, &jsonl).unwrap();
            eprintln!(
                "golden {name}: updated ({} -> {} bytes)",
                existing.len(),
                bytes.len()
            );
        }
        Ok(existing) => {
            let old: Vec<Event> = postcard::from_bytes(&existing).unwrap_or_default();
            let first_diff = old
                .iter()
                .zip(events)
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| old.len().min(events.len()));
            panic!(
                "golden {name} differs: {} events on disk vs {} produced; first difference at \
                 event {first_diff} ({}). Set UPDATE_GOLDEN=1 to accept and explain it in the PR.",
                old.len(),
                events.len(),
                events.get(first_diff).map_or("<end>", Event::kind)
            );
        }
        Err(_) => {
            std::fs::write(&bin_path, &bytes).unwrap();
            std::fs::write(&jsonl_path, &jsonl).unwrap();
            eprintln!("golden {name}: created; commit tests/golden/{name}.*");
        }
    }
}
