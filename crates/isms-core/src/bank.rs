//! The Public Investment Bank (GDD §6.5, §5 A5 `public-bank`; TDD §5.5 step
//! 8b, S0.17c). A seeded society-owned org whose treasury is the levy pool
//! (Q92): every cycle end each cooperative pays a capital levy on the value
//! of its installed Machines into the pool, the pool pays the society's basic
//! provision (a money floor, as the Republic's treasury does), and what is
//! left is lent by formula to cooperatives that applied, first come first
//! served, one loan per coop, at a fixed rate and term (Q94). Loans are
//! ordinary credit contracts with the bank as lender, so installments and
//! defaults follow the S0.11 rules and flow back to the pool.

use crate::command::{Command, Envelope, Reject, RejectCode};
use crate::constitution::CapitalMode;
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::OrgId;
use crate::kinds::{Good, OrgKind};
use crate::ledger::Party;
use crate::money::Money;
use crate::tick::TickBuilder;
use crate::world::{ContractBody, ContractStatus, Instrument, OfferBody, Ownership, World};

/// The bank: the society-owned association seeded when capital is `public_bank`.
#[must_use]
pub fn bank_org(world: &World) -> Option<OrgId> {
    world
        .orgs
        .values()
        .find(|o| o.kind == OrgKind::Association && o.ownership == Ownership::Society)
        .map(|o| o.id)
}

/// The value of a coop's installed Machines at the last Machines price (the
/// trade, else the start price; Q93). Machines in inventory are stock, not capital.
#[must_use]
pub fn machine_value(world: &World, org: OrgId) -> Money {
    let Some(o) = world.orgs.get(&org) else {
        return Money::ZERO;
    };
    let machines: u32 = o
        .workplaces
        .iter()
        .filter_map(|w| world.workplaces.get(w))
        .map(|w| w.machines)
        .sum();
    let price =
        crate::market::last_price(world, Instrument::Good(Good::Machines)).unwrap_or(Money::ZERO);
    Money(price.0 * i64::from(machines))
}

/// The capital levy due from a cooperative at this cycle end.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn levy_due(world: &World, org: OrgId) -> Money {
    if world.constitution.capital != CapitalMode::PublicBank
        || !crate::coop::is_coop(world, org)
        || bank_org(world).is_none()
    {
        return Money::ZERO;
    }
    let rate = world.policy.capital_levy.unwrap_or(0.0);
    Money((machine_value(world, org).0 as f64 * rate).floor() as i64)
}

/// A coop's open loan from the bank, if any.
#[must_use]
pub fn open_bank_loan(world: &World, bank: OrgId, org: OrgId) -> bool {
    world.contracts.values().any(|k| {
        k.status == ContractStatus::Active
            && k.parties.0 == Party::Org(bank)
            && k.parties.1 == Party::Org(org)
            && matches!(k.body, ContractBody::Credit { .. })
    })
}

/// A coop's pending application, if any.
#[must_use]
pub fn pending_application(world: &World, org: OrgId) -> bool {
    world
        .offers
        .values()
        .any(|f| matches!(f.body, OfferBody::BankLoan { org: x, .. } if x == org))
}

/// `RequestBankLoan`: a cooperative's manager applies for a loan.
pub fn request_bank_loan(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    principal: Money,
    term_cycles: u32,
) -> Result<Vec<Event>, Reject> {
    let o = crate::orgs::managed_org(world, envelope, org)?;
    if o.kind != OrgKind::Cooperative {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            "the bank lends to cooperatives",
        ));
    }
    let Some(bank) = bank_org(world) else {
        return Err(Reject::new(RejectCode::NoStore, "this society has no bank"));
    };
    let p = &world.params.bank;
    if principal <= Money::ZERO || principal > p.max_loan {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            format!("a loan is 0 < principal <= {}", p.max_loan),
        ));
    }
    if term_cycles == 0 || term_cycles > p.max_term_cycles {
        return Err(Reject::new(
            RejectCode::InvalidTerm,
            format!("a loan runs 1..={} cycles", p.max_term_cycles),
        ));
    }
    if pending_application(world, org) || open_bank_loan(world, bank, org) {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            "one application or loan at a time",
        ));
    }
    Ok(vec![Event::BankLoanRequested {
        offer: world.next.offer,
        org,
        principal,
        term_cycles,
    }])
}

/// Step 8b for public-bank systems: the levy into the pool, the basic
/// provision out of it, then lending by formula.
pub fn cycle_end_8b_levy_floor_and_lending(b: &mut TickBuilder) {
    if b.rules.capabilities.capital != CapitalMode::PublicBank {
        return;
    }
    let Some(bank) = bank_org(&b.world) else {
        return;
    };
    // 1. The levy.
    let coops: Vec<OrgId> = b
        .world
        .orgs
        .values()
        .filter(|o| o.kind == OrgKind::Cooperative)
        .map(|o| o.id)
        .collect();
    let rate = b.world.policy.capital_levy.unwrap_or(0.0);
    for org in coops {
        let due = levy_due(&b.world, org);
        let amount = due.min(b.world.orgs[&org].treasury);
        if amount <= Money::ZERO {
            continue;
        }
        let value = machine_value(&b.world, org);
        let explain = Explain::new(RuleId::CapitalLevy, "machine_value x capital_levy", amount)
            .input("machine_value", value)
            .input("capital_levy", rate)
            .input("due", due);
        b.emit(Event::LevyPaid {
            org,
            bank,
            amount,
            explain,
        });
    }
    // 2. Basic provision: the society's Food floor, in money, from the pool (Q92).
    if let Some(floor_food) = b.world.policy.minimum_food_ration
        && floor_food > 0
    {
        crate::tax::need_floor(b, crate::ledger::Holder::Org(bank), floor_food);
    }
    // 3. Lending by formula (Q94): applications in order, while the pool covers them.
    let applications: Vec<(crate::ids::OfferId, OrgId, Money, u32)> = b
        .world
        .offers
        .values()
        .filter_map(|f| match f.body {
            OfferBody::BankLoan {
                org,
                principal,
                term_cycles,
            } => Some((f.id, org, principal, term_cycles)),
            _ => None,
        })
        .collect();
    let rate_bp = b.world.params.bank.rate_per_cycle_bp;
    for (application, org, principal, term_cycles) in applications {
        let Some(o) = b.world.orgs.get(&org) else {
            continue;
        };
        if open_bank_loan(&b.world, bank, org) || o.payment_missed {
            b.emit(Event::BankLoanDecided {
                application,
                org,
                granted: false,
            });
            continue;
        }
        if b.world.orgs[&bank].treasury < principal {
            continue; // the application waits for the pool
        }
        let offer = b.world.next.offer;
        b.emit(Event::CreditOffered {
            offer,
            by: Party::Org(bank),
            body: OfferBody::Credit {
                to: Some(Party::Org(org)),
                principal,
                rate_per_cycle_bp: rate_bp,
                term_cycles,
                collateral: None,
            },
        });
        let (_, installment, _) = crate::credit::schedule(principal, rate_bp, term_cycles);
        b.emit(Event::CreditAccepted {
            contract: b.world.next.contract,
            lender: Party::Org(bank),
            borrower: Party::Org(org),
            principal,
            rate_per_cycle_bp: rate_bp,
            installment,
            installments: term_cycles,
            collateral: None,
        });
        b.emit(Event::BankLoanDecided {
            application,
            org,
            granted: true,
        });
    }
}

/// Principal still owed to the bank across its open loans (for the aggregates).
#[must_use]
pub fn loans_outstanding(world: &World) -> Money {
    let Some(bank) = bank_org(world) else {
        return Money::ZERO;
    };
    world
        .contracts
        .values()
        .filter(|k| k.status == ContractStatus::Active && k.parties.0 == Party::Org(bank))
        .filter_map(|k| match k.body {
            ContractBody::Credit {
                installment,
                installments_left,
                ..
            } => Some(Money(installment.0 * i64::from(installments_left))),
            _ => None,
        })
        .sum()
}

// ---------------------------------------------------------------------------
// Admission by vote (GDD §6.5 "coops admit new members by vote"; Q95).

/// `ProposeAdmission`: a member proposes a candidate; the proposer's vote is a yes.
pub fn propose_admission(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    citizen: crate::ids::CitizenId,
) -> Result<Vec<Event>, Reject> {
    let actor = crate::command::acting_citizen(world, envelope)?;
    let o = world
        .orgs
        .get(&org)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {org}")))?;
    if o.kind != OrgKind::Cooperative {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            format!("{} is not a cooperative", o.name),
        ));
    }
    if !o.members.contains(&actor.id) {
        return Err(Reject::new(RejectCode::NotParty, "only a member proposes"));
    }
    if !world.citizens.contains_key(&citizen) {
        return Err(Reject::new(
            RejectCode::UnknownCitizen,
            format!("no citizen {citizen}"),
        ));
    }
    if o.members.contains(&citizen) || crate::labor::has_position(world, citizen) {
        return Err(Reject::new(
            RejectCode::AlreadyExists,
            "already a member, or placed elsewhere",
        ));
    }
    if world.proposals.values().any(|p| {
        matches!(p.kind, crate::world::ProposalKind::Admission { org: x, citizen: c } if x == org && c == citizen)
    }) {
        return Err(Reject::new(RejectCode::AlreadyExists, "already proposed"));
    }
    let proposal = world.next.proposal;
    let mut events = vec![Event::AdmissionProposed {
        proposal,
        org,
        citizen,
        by: actor.id,
    }];
    // One member: the proposer's yes is the majority.
    if o.members.len() == 1 {
        events.push(Event::ProposalClosed {
            proposal,
            passed: true,
        });
        events.extend(admit_events(world, o, citizen));
    }
    Ok(events)
}

/// `VoteAdmission`: a member votes; a majority of the members admits.
pub fn vote_admission(
    world: &World,
    envelope: &Envelope<Command>,
    proposal: crate::ids::ProposalId,
    approve: bool,
) -> Result<Vec<Event>, Reject> {
    let actor = crate::command::acting_citizen(world, envelope)?;
    let p = world.proposals.get(&proposal).ok_or_else(|| {
        Reject::new(
            RejectCode::UnknownOffer,
            format!("no open proposal {proposal}"),
        )
    })?;
    let crate::world::ProposalKind::Admission { org, citizen } = p.kind;
    let o = &world.orgs[&org];
    if !o.members.contains(&actor.id) {
        return Err(Reject::new(RejectCode::NotParty, "only a member votes"));
    }
    if p.votes.contains_key(&actor.id) {
        return Err(Reject::new(RejectCode::AlreadyExists, "already voted"));
    }
    let mut events = vec![Event::AdmissionVoted {
        proposal,
        citizen: actor.id,
        approve,
    }];
    let yes = p.votes.values().filter(|v| **v).count() + usize::from(approve);
    let no = p.votes.values().filter(|v| !**v).count() + usize::from(!approve);
    let members = o.members.len();
    if yes * 2 > members {
        events.push(Event::ProposalClosed {
            proposal,
            passed: true,
        });
        events.extend(admit_events(world, o, citizen));
    } else if no * 2 >= members {
        events.push(Event::ProposalClosed {
            proposal,
            passed: false,
        });
    }
    Ok(events)
}

fn admit_events(
    world: &World,
    o: &crate::world::Org,
    citizen: crate::ids::CitizenId,
) -> Vec<Event> {
    match crate::coop::least_crowded(world, o) {
        Some(workplace) if !crate::labor::has_position(world, citizen) => vec![
            Event::MemberAdmitted { org: o.id, citizen },
            Event::Assigned {
                workplace,
                citizen,
                contract: None,
            },
        ],
        _ => Vec::new(),
    }
}

/// Step 8j: admission proposals still open at cycle end lapse (votes close
/// each cycle).
pub fn cycle_end_8j_close_proposals(b: &mut TickBuilder) {
    let open: Vec<crate::ids::ProposalId> = b
        .world
        .proposals
        .values()
        .filter(|p| matches!(p.kind, crate::world::ProposalKind::Admission { .. }))
        .map(|p| p.id)
        .collect();
    for proposal in open {
        b.emit(Event::ProposalClosed {
            proposal,
            passed: false,
        });
    }
}
