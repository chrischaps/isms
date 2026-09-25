#![allow(
    clippy::many_single_char_names,
    clippy::doc_markdown,
    clippy::too_many_lines
)]
//! S0.10b done gate (TDD §18.2 S0.10, equity part): issuance into OrgSelf,
//! share sales on the book and direct, dividends by share count (none without
//! a controlling owner), control changes the manager, net worth and self-made.

use isms_core::command::{Command, Envelope, RejectCode};
use isms_core::event::Event;
use isms_core::ids::{OfferId, OrgId, WorkplaceId};
use isms_core::kinds::{Good, OrgKind, WorkplaceKind};
use isms_core::ledger::{Asset, Holder, Party};
use isms_core::money::Money;
use isms_core::shares::{book_value, citizen_held, net_worth, self_made, share_value, shares_held};
use isms_core::test_support::strategies::nth;
use isms_core::test_support::{Harness, WorldBuilder};
use isms_core::world::{Instrument, Price, SaleAsset, Side};

/// A legacy firm (100% OrgSelf, householder manager) with a Mine, 2 machines and Ore.
fn legacy() -> Harness {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(3)
        .householders(1)
        .org(OrgKind::Firm, "Legacy Mine")
        .workplace(WorkplaceKind::Mine, 0, 2)
        .org_inventory(0, Good::Ore, 218)
        .build();
    // quiet plans: no automatic Food bids sitting in escrow
    for who in h.citizen_ids() {
        let mut p = h.citizen(who).plan.clone();
        p.keep_food_at_least = 0;
        h.apply(Event::PlanChanged {
            citizen: who,
            plan: Box::new(p),
        });
    }
    let hh = nth(&h, 3);
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(hh),
    });
    h.apply(Event::Seeded {
        holder: Holder::Org(OrgId(0)),
        asset: Asset::Money(Money::cents(41230)),
    });
    h
}

#[test]
fn book_value_and_share_value_follow_the_wireframe() {
    let h = legacy();
    let org = &h.world.orgs[&OrgId(0)];
    // treasury 412.30 + 218 Ore @ 0.90 + 2 machines @ 9.00 (start prices)
    assert_eq!(
        book_value(&h.world, org),
        Money::cents(41230 + 218 * 90 + 2 * 900)
    );
    assert_eq!(
        share_value(&h.world, org),
        Money::cents((41230 + 218 * 90 + 1800) / 100)
    );
    assert_eq!(citizen_held(org), 0);
    assert_eq!(net_worth(&h.world, nth(&h, 0)), Money::credits(1000));
    assert_eq!(self_made(&h.world, nth(&h, 0)), Money::ZERO);
}

#[test]
fn buying_control_on_the_book_makes_the_buyer_manager() {
    let mut h = legacy();
    let (buyer, hh) = (nth(&h, 0), nth(&h, 3));
    // the householder manager lists 100% at book value per share on behalf of the org
    let per_share = share_value(&h.world, &h.world.orgs[&OrgId(0)]);
    h.cmd(
        Envelope::citizen(
            hh,
            Command::PlaceOrder {
                instrument: Instrument::Share(OrgId(0)),
                side: Side::Ask,
                qty: 100,
                limit_price: per_share,
                expires_tick: Some(5000),
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    assert_eq!(
        shares_held(&h.world, Party::Org(OrgId(0)), OrgId(0)),
        0,
        "escrowed"
    );
    assert_eq!(h.world.share_escrow.len(), 1);
    // a bid for 60 crosses
    let events = h
        .cmd(Envelope::citizen(
            buyer,
            Command::PlaceOrder {
                instrument: Instrument::Share(OrgId(0)),
                side: Side::Bid,
                qty: 60,
                limit_price: per_share,
                expires_tick: None,
            },
            0,
        ))
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Trade { qty: 60, .. }))
    );
    assert!(
        events.iter().any(
            |e| matches!(e, Event::ManagerAppointed { citizen, .. } if *citizen == Some(buyer))
        ),
        "control passed"
    );
    assert_eq!(shares_held(&h.world, Party::Citizen(buyer), OrgId(0)), 60);
    assert_eq!(h.world.orgs[&OrgId(0)].manager, Some(buyer));
    assert_eq!(
        h.world.orgs[&OrgId(0)].treasury,
        Money::cents(41230) + Money(per_share.0 * 60)
    );
    assert_eq!(
        h.citizen(buyer).household.balance,
        Money::credits(1000) - Money(per_share.0 * 60)
    );
    // net worth now counts the shares at the last trade price
    assert_eq!(net_worth(&h.world, buyer), Money::credits(1000));
    h.check();
}

#[test]
fn issuance_lands_in_orgself_and_direct_sale_moves_shares() {
    let mut h = legacy();
    let (a, b) = (nth(&h, 0), nth(&h, 1));
    // a buys 100% by direct sale from the org (manager offers)
    let hh = nth(&h, 3);
    h.cmd(
        Envelope::citizen(
            hh,
            Command::OfferSale {
                asset: SaleAsset::Shares(OrgId(0), 100),
                price: Price::Money(Money::credits(600)),
                to: None,
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    assert_eq!(h.world.share_escrow.len(), 1);
    let r = h.cmd_dry(Envelope::citizen(
        a,
        Command::IssueShares {
            org: OrgId(0),
            qty: 10,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::NotControllingOwner);
    let events = h
        .cmd(Envelope::citizen(
            a,
            Command::AcceptSale { offer: OfferId(0) },
            0,
        ))
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::ManagerAppointed { citizen, .. } if *citizen == Some(a)))
    );
    assert_eq!(shares_held(&h.world, Party::Citizen(a), OrgId(0)), 100);
    assert!(h.world.share_escrow.is_empty());
    h.cmd(Envelope::citizen(
        a,
        Command::IssueShares {
            org: OrgId(0),
            qty: 50,
        },
        0,
    ))
    .unwrap();
    assert_eq!(shares_held(&h.world, Party::Org(OrgId(0)), OrgId(0)), 50);
    assert!(matches!(
        h.world.orgs[&OrgId(0)].ownership,
        isms_core::world::Ownership::Shares { issued: 150, .. }
    ));
    // a sells 30 to b directly; a keeps control (70 of 150 is not > 50%... 70*2 = 140 < 150)
    h.cmd(Envelope::citizen(
        a,
        Command::OfferSale {
            asset: SaleAsset::Shares(OrgId(0), 30),
            price: Price::Money(Money::credits(200)),
            to: Some(Party::Citizen(b)),
        },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptSale { offer: OfferId(1) },
        0,
    ))
    .unwrap();
    assert_eq!(shares_held(&h.world, Party::Citizen(a), OrgId(0)), 70);
    assert_eq!(shares_held(&h.world, Party::Citizen(b), OrgId(0)), 30);
    assert_eq!(
        isms_core::orgs::controlling_owner(&h.world.orgs[&OrgId(0)]),
        None
    );
    h.check();
}

#[test]
fn dividends_split_by_share_count_and_need_a_controlling_owner() {
    let mut h = legacy();
    let (a, b, hh) = (nth(&h, 0), nth(&h, 1), nth(&h, 3));
    let r = h.cmd_dry(
        Envelope::citizen(
            hh,
            Command::DeclareDividend {
                org: OrgId(0),
                per_share: Money::cents(10),
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    );
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::NotControllingOwner,
        "OrgSelf never controls"
    );
    h.cmd(
        Envelope::citizen(
            hh,
            Command::OfferSale {
                asset: SaleAsset::Shares(OrgId(0), 100),
                price: Price::Money(Money::credits(100)),
                to: None,
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    h.cmd(Envelope::citizen(
        a,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        a,
        Command::OfferSale {
            asset: SaleAsset::Shares(OrgId(0), 25),
            price: Price::Money(Money::credits(1)),
            to: Some(Party::Citizen(b)),
        },
        0,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptSale { offer: OfferId(1) },
        0,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        a,
        Command::DeclareDividend {
            org: OrgId(0),
            per_share: Money::credits(100),
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::InsufficientFunds);
    h.cmd(Envelope::citizen(
        a,
        Command::DeclareDividend {
            org: OrgId(0),
            per_share: Money::cents(200),
        },
        0,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        a,
        Command::DeclareDividend {
            org: OrgId(0),
            per_share: Money::cents(1),
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::AlreadyExists);
    let (ba, bb) = (
        h.citizen(a).household.balance,
        h.citizen(b).household.balance,
    );
    h.check_every_step = false;
    let events = h.run_cycle();
    let paid: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::DividendPaid {
                citizen, amount, ..
            } => Some((*citizen, *amount)),
            _ => None,
        })
        .collect();
    assert_eq!(
        paid,
        vec![(a, Money::cents(200 * 75)), (b, Money::cents(200 * 25))]
    );
    assert_eq!(h.citizen(a).household.balance, ba + Money::cents(15000));
    assert_eq!(h.citizen(b).household.balance, bb + Money::cents(5000));
    assert!(
        h.world.orgs[&OrgId(0)].declared_dividend.is_none(),
        "cleared at cycle close"
    );
    // now nobody controls (a has 75 of 100? yes 75 > 50) -> sell down to 50
    h.cmd(Envelope::citizen(
        a,
        Command::OfferSale {
            asset: SaleAsset::Shares(OrgId(0), 25),
            price: Price::Money(Money::credits(1)),
            to: Some(Party::Citizen(b)),
        },
        24,
    ))
    .unwrap();
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptSale { offer: OfferId(2) },
        24,
    ))
    .unwrap();
    let r = h.cmd_dry(Envelope::citizen(
        a,
        Command::DeclareDividend {
            org: OrgId(0),
            per_share: Money::cents(1),
        },
        24,
    ));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::NotControllingOwner,
        "50% is not control (T21)"
    );
    h.check();
}

#[test]
fn share_orders_validate_holdings_and_cancel_releases_shares() {
    let mut h = legacy();
    let a = nth(&h, 0);
    let r = h.cmd_dry(Envelope::citizen(
        a,
        Command::PlaceOrder {
            instrument: Instrument::Share(OrgId(0)),
            side: Side::Ask,
            qty: 1,
            limit_price: Money::cents(100),
            expires_tick: None,
        },
        0,
    ));
    assert_eq!(
        r.unwrap_err().code,
        RejectCode::InsufficientGoods,
        "holds no shares"
    );
    let r = h.cmd_dry(Envelope::citizen(
        a,
        Command::PlaceOrder {
            instrument: Instrument::Share(OrgId(7)),
            side: Side::Bid,
            qty: 1,
            limit_price: Money::cents(100),
            expires_tick: None,
        },
        0,
    ));
    assert_eq!(r.unwrap_err().code, RejectCode::UnknownOrg);
    let hh = nth(&h, 3);
    h.cmd(
        Envelope::citizen(
            hh,
            Command::PlaceOrder {
                instrument: Instrument::Share(OrgId(0)),
                side: Side::Ask,
                qty: 40,
                limit_price: Money::cents(100),
                expires_tick: None,
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    assert_eq!(shares_held(&h.world, Party::Org(OrgId(0)), OrgId(0)), 60);
    h.cmd(
        Envelope::citizen(
            hh,
            Command::CancelOrder {
                order: isms_core::ids::OrderId(0),
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    assert_eq!(shares_held(&h.world, Party::Org(OrgId(0)), OrgId(0)), 100);
    assert!(h.world.share_escrow.is_empty());
    let _ = WorkplaceId(0);
    h.check();
}

/// Two humans, a householder managing "Legacy Builders" with two dwellings
/// and no treasury; quiet plans (E-2).
fn builders() -> Harness {
    let mut h = WorldBuilder::new("freeport")
        .with_preset(|p| p.params.population.collapse_enabled = false)
        .humans(2)
        .householders(1)
        .org(OrgKind::Firm, "Legacy Builders")
        .dwelling(0)
        .dwelling(0)
        .build();
    for who in h.citizen_ids() {
        let mut p = h.citizen(who).plan.clone();
        p.keep_food_at_least = 0;
        h.apply(Event::PlanChanged {
            citizen: who,
            plan: Box::new(p),
        });
    }
    h.apply(Event::ManagerAppointed {
        org: OrgId(0),
        citizen: Some(nth(&h, 2)),
    });
    h
}

#[test]
fn a_dwelling_held_counts_at_its_build_cost_until_one_sells() {
    use isms_core::ids::DwellingId;
    use isms_core::shares::{dwelling_value, dwellings_value};
    use isms_core::world::Owner;
    let mut h = builders();
    let (buyer, mgr) = (nth(&h, 0), nth(&h, 2));
    // No dwelling has sold: 10 Materials at 2.00 plus two hours at the legacy wage 8.00.
    assert_eq!(dwelling_value(&h.world), Money::credits(36));
    assert_eq!(
        dwellings_value(&h.world, Owner::Org(OrgId(0))),
        Money::credits(72)
    );
    assert_eq!(
        book_value(&h.world, &h.world.orgs[&OrgId(0)]),
        Money::credits(72),
        "the dwellings a Builder holds are its book"
    );
    assert_eq!(net_worth(&h.world, buyer), Money::credits(1000));
    h.cmd(
        Envelope::citizen(
            mgr,
            Command::OfferSale {
                asset: SaleAsset::Dwelling(DwellingId(0)),
                price: Price::Money(Money::credits(50)),
                to: None,
            },
            0,
        )
        .on_behalf_of(OrgId(0)),
    )
    .unwrap();
    h.cmd(Envelope::citizen(
        buyer,
        Command::AcceptSale { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    // The sale sets the price every dwelling in the society is worth.
    assert_eq!(h.world.meta.last_dwelling_price, Some(Money::credits(50)));
    assert_eq!(dwelling_value(&h.world), Money::credits(50));
    assert_eq!(
        net_worth(&h.world, buyer),
        Money::credits(1000),
        "950 in hand and a dwelling at the price paid"
    );
    assert_eq!(self_made(&h.world, buyer), Money::ZERO);
    assert_eq!(
        book_value(&h.world, &h.world.orgs[&OrgId(0)]),
        Money::credits(100),
        "50 in the treasury and one dwelling at 50"
    );
    h.check();
}

#[test]
fn a_loan_counts_for_the_lender_and_against_the_borrower_at_the_unpaid_principal() {
    use isms_core::credit::outstanding;
    let mut h = builders();
    let (l, b) = (nth(&h, 0), nth(&h, 1));
    h.cmd(Envelope::citizen(
        l,
        Command::OfferCredit {
            to: Some(Party::Citizen(b)),
            principal: Money::credits(100),
            rate_per_cycle_bp: 200,
            term_cycles: 5,
            collateral: None,
        },
        0,
    ))
    .unwrap();
    assert_eq!(
        net_worth(&h.world, l),
        Money::credits(900),
        "escrowed, not yet lent"
    );
    h.cmd(Envelope::citizen(
        b,
        Command::AcceptCredit { offer: OfferId(0) },
        0,
    ))
    .unwrap();
    assert_eq!(
        outstanding(&h.world, Party::Citizen(l)),
        (Money::credits(100), Money::ZERO)
    );
    assert_eq!(
        outstanding(&h.world, Party::Citizen(b)),
        (Money::ZERO, Money::credits(100))
    );
    assert_eq!(
        net_worth(&h.world, l),
        Money::credits(1000),
        "the loan is a holding"
    );
    assert_eq!(
        net_worth(&h.world, b),
        Money::credits(1000),
        "the money in hand is owed"
    );
    h.check_every_step = false;
    let events = h.run_cycle();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::CreditInstallment { .. }))
    );
    // One of five installments paid: four fifths of the principal stand.
    assert_eq!(
        outstanding(&h.world, Party::Citizen(l)),
        (Money::credits(80), Money::ZERO)
    );
    assert_eq!(
        outstanding(&h.world, Party::Citizen(b)),
        (Money::ZERO, Money::credits(80))
    );
    let pantry = |who| {
        let c = h.citizen(who);
        c.household
            .pantry
            .get(&Good::Food)
            .map_or(Money::ZERO, |q| Money(130 * i64::from(*q)))
    };
    assert_eq!(
        net_worth(&h.world, l) - h.citizen(l).household.balance - pantry(l),
        Money::credits(80),
        "the interest arrived as balance; the principal is still out"
    );
    h.check();
}
