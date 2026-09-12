//! Per-preset lexicon (GDD §15): concept key → display string. The engine only
//! validates completeness; rendering is the client's job.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum LexiconError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("lexicon for {preset} is missing keys: {missing:?}")]
    MissingKeys {
        preset: String,
        missing: Vec<String>,
    },
}

/// The keys every lexicon must define, from `presets/lexicon/KEYS.txt`
/// (blank lines and `#` comments ignored).
pub fn required_keys(presets_dir: &Path) -> Result<Vec<String>, LexiconError> {
    let path = presets_dir.join("lexicon").join("KEYS.txt");
    let text =
        std::fs::read_to_string(&path).map_err(|source| LexiconError::Io { path, source })?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

/// Load `<presets_dir>/lexicon/<preset>.json` and check it defines every required key.
/// Keys starting with `_` are comments and are dropped.
pub fn load_lexicon(
    presets_dir: &Path,
    preset: &str,
) -> Result<BTreeMap<String, String>, LexiconError> {
    let path = presets_dir.join("lexicon").join(format!("{preset}.json"));
    let text = std::fs::read_to_string(&path).map_err(|source| LexiconError::Io {
        path: path.clone(),
        source,
    })?;
    let raw: BTreeMap<String, String> =
        serde_json::from_str(&text).map_err(|source| LexiconError::Parse { path, source })?;
    let lexicon: BTreeMap<String, String> = raw
        .into_iter()
        .filter(|(k, _)| !k.starts_with('_'))
        .collect();
    let missing: Vec<String> = required_keys(presets_dir)?
        .into_iter()
        .filter(|k| !lexicon.contains_key(k))
        .collect();
    if !missing.is_empty() {
        return Err(LexiconError::MissingKeys {
            preset: preset.to_owned(),
            missing,
        });
    }
    Ok(lexicon)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> PathBuf {
        PathBuf::from(crate::WORKSPACE_PRESETS_DIR)
    }

    #[test]
    fn every_preset_defines_every_lexicon_key() {
        let presets = crate::config::load_all(&dir()).unwrap();
        assert_eq!(presets.len(), 5);
        for p in &presets {
            let lex = load_lexicon(&dir(), &p.name).unwrap_or_else(|e| panic!("{e}"));
            assert!(!lex.is_empty());
        }
    }

    #[test]
    fn freeport_lexicon_is_real_copy() {
        let lex = load_lexicon(&dir(), "freeport").unwrap();
        assert_eq!(lex["compensation"], "Payslip");
        assert_eq!(lex["home_title"], "Your Accounts");
    }

    #[test]
    fn missing_key_is_reported_by_name() {
        let tmp = std::env::temp_dir().join(format!("isms-lex-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("lexicon")).unwrap();
        std::fs::copy(dir().join("lexicon/KEYS.txt"), tmp.join("lexicon/KEYS.txt")).unwrap();
        std::fs::write(tmp.join("lexicon/x.json"), r#"{"compensation": "Pay"}"#).unwrap();
        let err = load_lexicon(&tmp, "x").unwrap_err();
        match err {
            LexiconError::MissingKeys { missing, .. } => {
                assert!(missing.contains(&"job".to_owned()));
                assert!(!missing.contains(&"compensation".to_owned()));
            }
            other => panic!("unexpected {other}"),
        }
        std::fs::remove_dir_all(&tmp).ok();
    }
}
