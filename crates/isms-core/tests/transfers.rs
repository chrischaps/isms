#![allow(clippy::many_single_char_names)]

//! S0.7 done gate (TDD §18.2): transfers and direct sales conserve, never go
//! negative, respect pantry caps; capability gating in the Commune; org treasury
//! access is the manager's alone.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::ids::{OfferId, OrgId};
use isms_core::kinds::{Good, OrgKind};
use isms_core::ledger::{Asset, Party};
use isms_core::money::Money;
use isms_core::test_support::strategies::{arb_cmd_scenario, nth, run_cmd_scenario};
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{Price, SaleAsset};
use proptest::prelude::*;

fn freeport(n: u32) -> Harness {
    let mut b = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(n)
        .org(OrgKind::Firm, "Legacy Mill");
    for i in 0..n as usize {
        b = b.pantry(i, Good::Food, 10).pantry(i, Good::Wares, 3);
    }
    b.org_inventory(0, Good::Food, 100).build()
}

fn money(to: Party, credits: i64) -> Command {
    Command::Transfer {
        to,
        asset: Asset::Money(Money::credits(credits)),
        memo: "x".into(),
    }
}

#[test]
fn transfers_move_money_and_goods_with_limits() {
    let mut h = freeport(2);
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(a, money(Party::Citizen(b), 250), 0))
        .unwrap();
    assert_eq!(h.citizen(a).household.balance, Money::credits(750));
    assert_eq!(h.citizen(b).household.balance, Money::credits(1250));
    let r = h.cmd_dry(Envelope::citizen(a, money(Party::Citizen(b), 751), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientFunds);
    let r = h.cmd_dry(Envelope::citizen(a, money(Party::Citizen(a), 1), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::SelfDeal);
    let r = h.cmd_dry(Envelope::citizen(
        a,
        money(Party::Citizen(isms_core::ids::CitizenId(9)), 1),
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::UnknownCitizen);
    // goods: b holds 10 Food, cap 48 -> a can give at most 38
    let give = |q| Command::Transfer {
        to: Party::Citizen(b),
        asset: Asset::Good(Good::Food, q),
        memo: String::new(),
    };
    let r = h.cmd_dry(Envelope::citizen(a, give(11), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientGoods);
    h.cmd(Envelope::citizen(a, give(10), 0)).unwrap();
    assert_eq!(h.citizen(b).household.pantry[&Good::Food], 20);
    h.apply(Event::Seeded {
        holder: isms_core::ledger::Holder::Citizen(a),
        asset: Asset::Good(Good::Food, 40),
    });
    let r = h.cmd_dry(Envelope::citizen(a, give(29), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::PantryFull);
    h.cmd(Envelope::citizen(a, give(28), 0)).unwrap();
    assert_eq!(h.citizen(b).household.pantry[&Good::Food], 48);
    // an org has no cap
    h.cmd(Envelope::citizen(
        a,
        Command::Transfer {
            to: Party::Org(OrgId(0)),
            asset: Asset::Good(Good::Food, 12),
            memo: String::new(),
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.world.orgs[&OrgId(0)].inventory[&Good::Food], 112);
    h.check();
}

#[test]
fn only_the_manager_moves_the_treasury() {
    let mut h = freeport(2);
    let (mgr, other) = (nth(&h, 0), nth(&h, 1));
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(mgr),
    });
    h.apply(Event::Seeded {
        holder: isms_core::ledger::Holder::Org(OrgId(0)),
        asset: Asset::Money(Money::credits(500)),
    });
    let r = h.cmd_dry(
        Envelope::citizen(other, money(Party::Citizen(other), 10), 0).on_behalf_of(OrgId(0)),
    );
    assert_eq!(r.unwrap_err().code, RejectCode::NotManager);
    h.cmd(Envelope::citizen(mgr, money(Party::Citizen(other), 10), 0).on_behalf_of(OrgId(0)))
        .unwrap();
    assert_eq!(h.world.orgs[&OrgId(0)].treasury, Money::credits(490));
    assert_eq!(h.citizen(other).household.balance, Money::credits(1010));
    // and goods from the inventory
    h.cmd(
        Envelope::citizen(
            mgr,
            Command::Transfer {
                to: Party::Citizen(other),
                asset: Asset::Good(Good::Food, 5),
                memo: String::new(),
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    assert_eq!(h.world.orgs[&OrgId(0)].inventory[&Good::Food], 95);
    h.check();
}

#[test]
fn commune_allows_goods_and_barter_but_not_money() {
    let mut h = WorldBuilder::new("commune")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(2)
        .pantry(0, Good::Wares, 5)
        .pantry(1, Good::Food, 20)
        .build();
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    let r = h.cmd_dry(Envelope::citizen(a, money(Party::Citizen(b), 1), 0));
    assert_eq!(r.unwrap_err().code, RejectCode::NotInThisSociety);
    h.cmd(Envelope::citizen(
        a,
        Command::Transfer {
            to: Party::Citizen(b),
            asset: Asset::Good(Good::Wares, 1),
            memo: "gift".into(),
        },
        0,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        a,
        Command::OfferSale {
            asset: SaleAsset::Good(Good::Wares, 2),
            price: Price::Money(Money::credits(1)),
            to: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotInThisSociety);
    let events = h
        .cmd(Envelope::citizen(
            a,
            Command::OfferSale {
                asset: SaleAsset::Good(Good::Wares, 2),
                price: Price::Good(Good::Food, 6),
                to: None,
            },
            0,
        ))
        .unwrap();
    assert!(matches!(
        events[0],
        Event::SaleOffered {
            offer: OfferId(0),
            ..
        }
    ));
    assert_eq!(h.citizen(a).household.pantry[&Good::Wares], 2, "escrowed");
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    assert_eq!(h.citizen(a).household.pantry[&Good::Food], 6);
    assert_eq!(h.citizen(b).household.pantry[&Good::Wares], 3);
    assert_eq!(h.citizen(b).household.pantry[&Good::Food], 14);
    assert!(h.world.offers.is_empty() && h.world.escrow.is_empty());
    h.check();
}

#[test]
fn direct_sale_escrows_settles_cancels_and_respects_addressee() {
    let mut h = freeport(3);
    let (s, b, c) = (nth(&h, 0), nth(&h, 1), nth(&h, 2));
    let offer = |to| Command::OfferSale {
        asset: SaleAsset::Good(Good::Wares, 2),
        price: Price::Money(Money::cents(410)),
        to,
    };
    let r = h.cmd_dry(Envelope::citizen(
        s,
        Command::OfferSale {
            asset: SaleAsset::Good(Good::Wares, 4),
            price: Price::Money(Money::cents(1)),
            to: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientGoods);
    h.cmd(Envelope::citizen(s, offer(Some(Party::Citizen(b))), 0))
        .unwrap();
    assert_eq!(h.world.escrow.len(), 1);
    assert_eq!(h.citizen(s).household.pantry[&Good::Wares], 1);
    let r = h.cmd_dry(Envelope::citizen(
        c,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotParty);
    let r = h.cmd_dry(Envelope::citizen(
        s,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::SelfDeal);
    let r = h.cmd_dry(Envelope::citizen(
        b,
        Command::CancelSale { offer: OfferId(0) },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotParty);
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    assert_eq!(h.citizen(b).household.pantry[&Good::Wares], 5);
    assert_eq!(
        h.citizen(b).household.balance,
        Money::credits(1000) - Money::cents(410)
    );
    assert_eq!(
        h.citizen(s).household.balance,
        Money::credits(1000) + Money::cents(410)
    );
    assert!(h.world.escrow.is_empty());
    // cancel releases exactly the escrow
    h.cmd(Envelope::citizen(
        s,
        Command::OfferSale {
            asset: SaleAsset::Good(Good::Wares, 1),
            price: Price::Money(Money::cents(500)),
            to: None,
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.citizen(s).household.pantry.get(&Good::Wares), None);
    h.cmd(Envelope::citizen(
        s,
        Command::CancelSale { offer: OfferId(1) },
        0,
    ))
    .unwrap();
    assert_eq!(h.citizen(s).household.pantry[&Good::Wares], 1);
    assert!(h.world.escrow.is_empty() && h.world.offers.is_empty());
    let r = h.cmd_dry(Envelope::citizen(
        b,
        Command::AcceptSale { offer: OfferId(1) },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::UnknownOffer);
    h.check();
}

#[test]
fn wanted_ads_have_no_mechanical_effect() {
    let mut h = freeport(2);
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(
        a,
        Command::PostWanted {
            good: Good::Machines,
            qty: 2,
            max_price: Money::credits(9),
        },
        0,
    ))
    .unwrap();
    assert_eq!(h.world.offers.len(), 1);
    assert!(h.world.escrow.is_empty());
    let r = h.cmd_dry(Envelope::citizen(
        b,
        Command::RemoveWanted { offer: OfferId(0) },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotParty);
    let r = h.cmd_dry(Envelope::citizen(
        b,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::UnknownOffer,
        "a wanted ad is not a sale"
    );
    h.cmd(Envelope::citizen(
        a,
        Command::RemoveWanted { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    assert!(h.world.offers.is_empty());
    h.check();
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// Arbitrary transfer and direct-sale command sequences (rejections ignored)
    /// never create or destroy money or goods, never go negative, never exceed
    /// pantry caps, and every escrow is released on cancel or accept.
    #[test]
    fn transfers_and_sales_conserve(steps in arb_cmd_scenario(4, 60)) {
        let mut h = freeport(4);
        h.check_every_step = false;
        run_cmd_scenario(&mut h, &steps);
        h.check();
        for c in h.world.citizens.values() {
            prop_assert!(!c.household.balance.is_negative());
            for (g, q) in &c.household.pantry {
                if let Some(cap) = h.world.params.pantry.get(g) {
                    prop_assert!(q <= cap, "{g:?}: {q} > {cap}");
                }
            }
        }
        // cancel everything that is still open and check escrow drains
        let open: Vec<(isms_core::ids::OfferId, Party)> = h.world.offers.values().map(|o| (o.id, o.by)).collect();
        for (id, by) in open {
            if let Party::Citizen(c) = by {
                let _ = h.cmd(Envelope::citizen(c, Command::CancelSale { offer: id }, 0));
                let _ = h.cmd(Envelope::citizen(c, Command::RemoveWanted { offer: id }, 0));
            }
        }
        prop_assert!(h.world.escrow.is_empty());
        h.check();
    }
}
