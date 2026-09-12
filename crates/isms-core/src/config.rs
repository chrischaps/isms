//! Preset loading: `_base.toml` overlaid by `<preset>.toml`, deserialized into a
//! validated [`Preset`]. The loaded struct is embedded in the society's first
//! event so a society's history is self-contained (TDD §7).

use crate::constitution::Constitution;
use crate::params::Params;
use crate::policy::Policy;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml::Table;
use toml::Value;

/// A fully loaded, validated preset: what `SocietyCreated` carries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub display: String,
    pub gdd_section: String,
    pub constitution: Constitution,
    pub policy: Policy,
    pub params: Params,
    pub scoreboard: Scoreboard,
}

/// The scoreboard fields the society shows (GDD §10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scoreboard {
    pub primary: Vec<String>,
}

/// The preset file's own top level, before params are merged.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    name: String,
    display: String,
    gdd_section: String,
    constitution: Constitution,
    policy: Policy,
    #[serde(default)]
    params: Table,
    scoreboard: Scoreboard,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BaseFile {
    params: Table,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
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
        source: toml::de::Error,
    },
    #[error("preset {name}: params override a non-table with a table at `{key}`")]
    OverlayShape { name: String, key: String },
    #[error("preset file name `{file}` does not match its `name = \"{name}\"`")]
    NameMismatch { file: String, name: String },
    #[error("preset {name} violates a constitutional constraint: {reason}")]
    Constraint { name: String, reason: String },
}

/// Load `<dir>/_base.toml` and `<dir>/<name>.toml`, overlay params, validate.
pub fn load_preset(dir: &Path, name: &str) -> Result<Preset, ConfigError> {
    let base_path = dir.join("_base.toml");
    let preset_path = dir.join(format!("{name}.toml"));
    let base: BaseFile = read_toml(&base_path)?;
    let file: PresetFile = read_toml(&preset_path)?;
    if file.name != name {
        return Err(ConfigError::NameMismatch {
            file: name.to_owned(),
            name: file.name,
        });
    }

    let mut merged = base.params;
    overlay(&mut merged, file.params, &file.name, "params")?;
    let params: Params = Value::Table(merged)
        .try_into()
        .map_err(|source| ConfigError::Parse {
            path: preset_path.clone(),
            source,
        })?;

    let preset = Preset {
        name: file.name,
        display: file.display,
        gdd_section: file.gdd_section,
        constitution: file.constitution,
        policy: file.policy,
        params,
        scoreboard: file.scoreboard,
    };
    validate(&preset)?;
    Ok(preset)
}

/// Every preset in the directory (`.toml` files not starting with `_`), by name.
pub fn load_all(dir: &Path) -> Result<Vec<Preset>, ConfigError> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map_err(|source| ConfigError::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let p = e.path();
            let stem = p.file_stem()?.to_str()?.to_owned();
            (p.extension()?.to_str()? == "toml" && !stem.starts_with('_')).then_some(stem)
        })
        .collect();
    names.sort();
    names.iter().map(|n| load_preset(dir, n)).collect()
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

/// Deep-merge `over` into `base`: tables merge recursively, anything else replaces.
fn overlay(base: &mut Table, over: Table, name: &str, prefix: &str) -> Result<(), ConfigError> {
    for (key, value) in over {
        let path = format!("{prefix}.{key}");
        match (base.get_mut(&key), value) {
            (Some(Value::Table(b)), Value::Table(o)) => overlay(b, o, name, &path)?,
            (Some(existing), Value::Table(_)) if !existing.is_table() => {
                return Err(ConfigError::OverlayShape {
                    name: name.to_owned(),
                    key: path,
                });
            }
            (_, v) => {
                base.insert(key, v);
            }
        }
    }
    Ok(())
}

/// Cross-axis constraints (TDD §7, QUESTIONS Q14). The full fixture of invalid
/// combinations is tested in S0.2b; the rules live here so the loader never
/// returns an inconsistent constitution.
fn validate(preset: &Preset) -> Result<(), ConfigError> {
    use crate::constitution::{
        CapitalMode, Compensation, Governance, LaborMode, Ownership, Pricing, Redistribution,
    };
    let c = &preset.constitution;
    let fail = |reason: &str| {
        Err(ConfigError::Constraint {
            name: preset.name.clone(),
            reason: reason.to_owned(),
        })
    };
    if c.compensation == Compensation::Need
        && (c.pricing != Pricing::None || c.redistribution != Redistribution::Total)
    {
        return fail("compensation=need requires pricing=none and redistribution=total");
    }
    if c.pricing == Pricing::None && c.compensation != Compensation::Need {
        return fail("pricing=none requires compensation=need");
    }
    if c.capital == CapitalMode::Open && c.ownership != Ownership::Private {
        return fail("capital=open requires ownership=private");
    }
    if c.ownership == Ownership::Collective && c.capital != CapitalMode::None {
        return fail("ownership=collective requires capital=none");
    }
    if c.labor == LaborMode::Assigned && c.governance != Governance::Committee {
        return fail("labor=assigned requires governance=committee");
    }
    if c.compensation == Compensation::Share && c.ownership != Ownership::Cooperative {
        return fail("compensation=share requires ownership=cooperative");
    }
    if c.governance == Governance::None
        && (c.redistribution != Redistribution::None || c.pricing == Pricing::Administered)
    {
        return fail("governance=none requires redistribution=none and pricing != administered");
    }
    if let Some(split) = preset.policy.materials_split
        && (split.wares + split.machines + split.dwellings - 1.0).abs() > 1e-9
    {
        return fail("materials_split must sum to 1");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::{Good, WorkplaceKind};
    use crate::money::Money;

    fn dir() -> PathBuf {
        PathBuf::from(crate::WORKSPACE_PRESETS_DIR)
    }

    #[test]
    fn all_five_presets_load() {
        let all = load_all(&dir()).expect("presets load");
        let names: Vec<&str> = all.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "commonwealth",
                "commune",
                "directorate",
                "freeport",
                "republic"
            ]
        );
    }

    #[test]
    fn freeport_params_are_base_plus_overlay() {
        let p = load_preset(&dir(), "freeport").unwrap();
        assert_eq!(p.params.money.endowment, Money::credits(1000));
        assert_eq!(p.params.money.start_prices[&Good::Food], Money::cents(130));
        assert_eq!(p.params.seeded_workplaces[&WorkplaceKind::Farm], 3);
        assert_eq!(p.params.initial_dwellings, 40);
        assert_eq!(p.params.land[&WorkplaceKind::Farm], 8);
        assert!(!p.params.land.contains_key(&WorkplaceKind::Mill));
        assert_eq!(
            p.params.recipes[&WorkplaceKind::Mill].consumes[&Good::Grain],
            1
        );
        assert!(p.constitution.has_money());
        assert_eq!(p.params.ticks_per_epoch(), 1008);
    }

    #[test]
    fn commune_has_no_money_and_a_policy() {
        let p = load_preset(&dir(), "commune").unwrap();
        assert!(!p.constitution.has_money());
        assert_eq!(p.policy.work_norm_hours, Some(6));
        assert_eq!(p.constitution.offices.len(), 1);
    }

    #[test]
    fn directorate_price_list_is_in_cents() {
        let p = load_preset(&dir(), "directorate").unwrap();
        let list = p.policy.price_list.as_ref().unwrap();
        assert_eq!(list[&Good::Food], Money::cents(120));
        assert_eq!(p.policy.wage_grades.as_ref().unwrap()[2], Money::cents(750));
    }

    #[test]
    fn overlay_replaces_scalars_and_merges_tables() {
        let mut base: Table = toml::from_str("a = 1\n[t]\nx = 1\ny = 2\n").unwrap();
        let over: Table = toml::from_str("a = 5\n[t]\ny = 9\nz = 3\n").unwrap();
        overlay(&mut base, over, "test", "params").unwrap();
        assert_eq!(base["a"].as_integer(), Some(5));
        let t = base["t"].as_table().unwrap();
        assert_eq!(t["x"].as_integer(), Some(1));
        assert_eq!(t["y"].as_integer(), Some(9));
        assert_eq!(t["z"].as_integer(), Some(3));
    }

    #[test]
    fn overlay_rejects_table_over_scalar() {
        let mut base: Table = toml::from_str("a = 1\n").unwrap();
        let over: Table = toml::from_str("[a]\nb = 1\n").unwrap();
        let err = overlay(&mut base, over, "test", "params").unwrap_err();
        assert!(matches!(err, ConfigError::OverlayShape { .. }), "{err}");
    }

    fn with_edited_presets(tag: &str, edit: impl FnOnce(&mut String, &mut String)) -> ConfigError {
        let tmp = std::env::temp_dir().join(format!("isms-cfg-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let mut base = std::fs::read_to_string(dir().join("_base.toml")).unwrap();
        let mut preset = std::fs::read_to_string(dir().join("freeport.toml")).unwrap();
        edit(&mut base, &mut preset);
        std::fs::write(tmp.join("_base.toml"), base).unwrap();
        std::fs::write(tmp.join("freeport.toml"), preset).unwrap();
        let err = load_preset(&tmp, "freeport").unwrap_err();
        std::fs::remove_dir_all(&tmp).ok();
        err
    }

    #[test]
    fn missing_base_field_fails_to_load() {
        let err = with_edited_presets("missing", |base, _| {
            assert!(base.contains("food_decay_per_tick = 4\n"));
            *base = base.replace("food_decay_per_tick = 4\n", "");
        });
        assert!(err.to_string().contains("food_decay_per_tick"), "{err}");
    }

    #[test]
    fn unknown_param_fails_to_load() {
        let err = with_edited_presets("unknown", |_, preset| {
            *preset = preset.replace("[params]\n", "[params]\nmystery_knob = 3\n");
        });
        assert!(err.to_string().contains("mystery_knob"), "{err}");
    }

    #[test]
    fn preset_round_trips_through_json() {
        let p = load_preset(&dir(), "republic").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let back: Preset = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
    fn freeport() -> Preset {
        load_preset(&dir(), "freeport").unwrap()
    }

    fn expect_constraint(mut p: Preset, edit: impl FnOnce(&mut Preset), needle: &str) {
        edit(&mut p);
        match validate(&p) {
            Err(ConfigError::Constraint { reason, .. }) => {
                assert!(reason.contains(needle), "got: {reason}");
            }
            other => panic!("expected constraint error containing {needle:?}, got {other:?}"),
        }
    }

    #[test]
    fn invalid_axis_combinations_are_rejected() {
        use crate::constitution::{
            CapitalMode, Compensation, Governance, LaborMode, Ownership, Pricing, Redistribution,
        };
        // 1. need-based pay with prices
        expect_constraint(
            freeport(),
            |p| p.constitution.compensation = Compensation::Need,
            "need",
        );
        // 2. no prices but contract pay
        expect_constraint(
            freeport(),
            |p| p.constitution.pricing = Pricing::None,
            "pricing=none",
        );
        // 3. open capital without private ownership
        expect_constraint(
            freeport(),
            |p| p.constitution.ownership = Ownership::Cooperative,
            "capital=open",
        );
        // 4. collective ownership with a capital market
        expect_constraint(
            freeport(),
            |p| {
                p.constitution.ownership = Ownership::Collective;
                p.constitution.capital = CapitalMode::PublicBank;
            },
            "collective",
        );
        // 5. assigned labor without a committee
        expect_constraint(
            freeport(),
            |p| p.constitution.labor = LaborMode::Assigned,
            "assigned",
        );
        // 6. share pay outside cooperatives
        expect_constraint(
            freeport(),
            |p| p.constitution.compensation = Compensation::Share,
            "share",
        );
        // 7. no governance but redistribution
        expect_constraint(
            freeport(),
            |p| p.constitution.redistribution = Redistribution::Provision,
            "governance=none",
        );
        // 8. a Materials split that does not sum to 1
        expect_constraint(
            freeport(),
            |p| {
                p.constitution.governance = Governance::Direct;
                p.policy.materials_split = Some(crate::policy::MaterialsSplit {
                    wares: 0.5,
                    machines: 0.5,
                    dwellings: 0.5,
                });
            },
            "materials_split",
        );
    }

    #[test]
    fn params_round_trip_through_toml() {
        let p = freeport();
        let text = toml::to_string(&p.params).unwrap();
        let back: Params = toml::from_str(&text).unwrap();
        assert_eq!(p.params, back);
    }

    #[test]
    fn preset_name_must_match_file() {
        let err = with_edited_presets("name", |_, preset| {
            *preset = preset.replace("name = \"freeport\"", "name = \"freeport-x\"");
        });
        assert!(matches!(err, ConfigError::NameMismatch { .. }), "{err}");
    }
}
