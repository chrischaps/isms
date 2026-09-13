//! S0.12a done gate (seeding and population): a seeded Freeport has its legacy
//! firms, dwellings, householders and managers; a human joining triggers
//! exactly one emigration at the next cycle end with conservation across the
//! burn; a dropping human count refills.

use isms_core::command::{Command, Envelope};
use isms_core::event::Event;
use isms_core::kinds::{CitizenKind, OrgKind, WorkplaceKind};
use isms_core::money::Money;
use isms_core::test_support::WorldBuilder;

#[test]
fn a_seeded_freeport_matches_the_preset() {
    let h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed_epoch()
        .build();
    assert_eq!(h.world.orgs.len(), 18, "3+3+3+3+3+1+2 legacy firms");
    assert!(
        h.world
            .orgs
            .values()
            .all(|o| o.kind == OrgKind::Firm && o.manager.is_some())
    );
    assert_eq!(h.world.workplaces.len(), 18);
    assert_eq!(
        h.world
            .workplaces
            .values()
            .filter(|w| w.kind == WorkplaceKind::Farm)
            .count(),
        3
    );
    assert!(
        h.world
            .workplaces
            .values()
            .filter(|w| w.kind == WorkplaceKind::Farm)
            .all(|w| w.slot.is_some())
    );
    assert_eq!(h.world.dwellings.len(), 40);
    let builders: Vec<_> = h
        .world
        .orgs
        .values()
        .filter(|o| o.name.starts_with("Legacy Builders"))
        .map(|o| o.id)
        .collect();
    assert_eq!(builders.len(), 2);
    assert!(
        h.world
            .dwellings
            .values()
            .all(|d| matches!(d.owner, isms_core::world::Owner::Org(o) if builders.contains(&o)))
    );
    assert_eq!(h.world.citizens.len(), 40);
    assert!(
        h.world
            .citizens
            .values()
            .all(|c| c.kind == CitizenKind::Householder)
    );
    assert_eq!(
        h.world.orgs[&isms_core::ids::OrgId(0)].treasury,
        Money::credits(384)
    );
    assert_eq!(
        h.world.ledger_meta.minted,
        Money::credits(384 * 18 + 1000 * 40)
    );
    assert_eq!(h.world.ledger_meta.dwellings_built, 40);
    // legacy inventory (Q45): every Mill starts with Food and Grain, every Foundry with Ore
    let mills: Vec<_> = h
        .world
        .orgs
        .values()
        .filter(|o| o.name.starts_with("Legacy Mill"))
        .collect();
    assert_eq!(mills.len(), 3);
    assert!(
        mills
            .iter()
            .all(|o| o.inventory[&isms_core::kinds::Good::Food] == 320
                && o.inventory[&isms_core::kinds::Good::Grain] == 120)
    );
    assert_eq!(
        h.world.ledger_meta.seeded[&isms_core::kinds::Good::Food],
        960
    );
    h.check();
}

#[test]
fn a_commune_seeds_collectives_without_money() {
    let h = WorldBuilder::new("commune").seed_epoch().build();
    assert!(h.world.orgs.values().all(|o| o.kind == OrgKind::Collective));
    assert_eq!(h.world.ledger_meta.minted, Money::ZERO);
    assert_eq!(h.world.citizens.len(), 40);
    h.check();
}

#[test]
fn a_human_joining_triggers_exactly_one_emigration_at_the_next_cycle_end() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed_epoch()
        .build();
    h.check_every_step = false;
    h.cmd(Envelope::system(
        Command::Join {
            handle: "ada".into(),
            kind: CitizenKind::Human,
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.world.citizens.len(), 41);
    let minted_before = h.world.ledger_meta.minted;
    let events = h.run_cycle();
    let gone: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::HouseholderEmigrated { .. }))
        .collect();
    assert_eq!(gone.len(), 1);
    let Event::HouseholderEmigrated {
        citizen,
        burned_money,
        ..
    } = gone[0]
    else {
        unreachable!()
    };
    assert_eq!(citizen.0, 39, "the most recently joined householder leaves");
    assert!(h.citizen(*citizen).dormant);
    assert_eq!(h.citizen(*citizen).household.balance, Money::ZERO);
    assert_eq!(h.world.ledger_meta.burned_money, *burned_money);
    assert_eq!(h.world.ledger_meta.minted, minted_before);
    let active_hh = h
        .world
        .citizens
        .values()
        .filter(|c| c.kind == CitizenKind::Householder && !c.dormant)
        .count();
    assert_eq!(active_hh, 39);
    // any org they managed has a new householder manager
    assert!(
        h.world
            .orgs
            .values()
            .all(|o| o.manager.is_some_and(|m| !h.citizen(m).dormant))
    );
    h.check();
    // a second cycle changes nothing
    let events = h.run_cycle();
    assert!(!events.iter().any(|e| matches!(
        e,
        Event::HouseholderEmigrated { .. } | Event::HouseholderJoined { .. }
    )));
}

#[test]
fn a_dormant_human_is_backfilled() {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .seed_epoch()
        .build();
    h.check_every_step = false;
    h.cmd(Envelope::system(
        Command::Join {
            handle: "ada".into(),
            kind: CitizenKind::Human,
        },
        0,
    ))
    .unwrap();
    h.run_cycle();
    let ada = h.citizen_ids()[40];
    h.apply(Event::CitizenDormant { citizen: ada });
    let events = h.run_cycle();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::HouseholderJoined { .. }))
            .count(),
        1
    );
    let active_hh = h
        .world
        .citizens
        .values()
        .filter(|c| c.kind == CitizenKind::Householder && !c.dormant)
        .count();
    assert_eq!(active_hh, 40);
    h.check();
}
