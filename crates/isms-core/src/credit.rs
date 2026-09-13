//! Credit contracts and associations (GDD §6.1 credit, §7.1 associations,
//! §7.2 credit row; TDD §5.3, §5.5 phase 7 and step 8c, T9, S0.11).
//! Simple interest, equal installments in integer cents with the remainder on
//! the last one, collateral seized on a missed installment, then a public flag.

use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::event::Event;
use crate::explain::{Explain, RuleId};
use crate::ids::{ContractId, OfferId, OrgId};
use crate::kinds::OrgKind;
use crate::ledger::{Asset, Party};
use crate::money::Money;
use crate::tick::TickBuilder;
use crate::transfers::{acting_party, check_has};
use crate::world::{Collateral, ContractBody, ContractStatus, OfferBody, World};

/// Total interest and the installment schedule: `total = principal x rate x term`,
/// `installment = floor(total / term)`, the last installment takes the remainder.
#[must_use]
pub fn schedule(
    principal: Money,
    rate_per_cycle_bp: u32,
    term_cycles: u32,
) -> (Money, Money, Money) {
    let interest =
        Money(principal.0 * i64::from(rate_per_cycle_bp) * i64::from(term_cycles) / 10_000);
    let total = principal + interest;
    let term = i64::from(term_cycles.max(1));
    let installment = Money(total.0 / term);
    let last = total - Money(installment.0 * (term - 1));
    (total, installment, last)
}

/// `OfferCredit`: the lender escrows the principal.
pub fn offer_credit(
    world: &World,
    envelope: &Envelope<Command>,
    to: Option<Party>,
    principal: Money,
    rate_per_cycle_bp: u32,
    term_cycles: u32,
    collateral: Option<Collateral>,
) -> Result<Vec<Event>, Reject> {
    let lender = acting_party(world, envelope)?;
    if term_cycles == 0 {
        return Err(Reject::new(
            RejectCode::InvalidTerm,
            "term must be positive",
        ));
    }
    check_has(world, lender, Asset::Money(principal))?;
    if let Some(t) = to
        && t == lender
    {
        return Err(Reject::new(RejectCode::SelfDeal, "cannot lend to yourself"));
    }
    Ok(vec![Event::CreditOffered {
        offer: world.next.offer,
        by: lender,
        body: OfferBody::Credit {
            to,
            principal,
            rate_per_cycle_bp,
            term_cycles,
            collateral,
        },
    }])
}

fn owns_collateral(world: &World, borrower: Party, collateral: Collateral) -> Result<(), Reject> {
    match collateral {
        Collateral::Dwelling(d) => {
            let dw = world.dwellings.get(&d).ok_or_else(|| {
                Reject::new(RejectCode::UnknownDwelling, format!("no dwelling {d}"))
            })?;
            if dw.owner != crate::housing::owner_of(borrower) {
                return Err(Reject::new(
                    RejectCode::NotOwner,
                    format!("{borrower:?} does not own {d}"),
                ));
            }
            if pledged(world, collateral) {
                return Err(Reject::new(
                    RejectCode::AlreadyExists,
                    format!("{d} is already pledged"),
                ));
            }
        }
        Collateral::Shares(org, qty) => {
            let held = crate::shares::shares_held(world, borrower, org);
            if held < qty || qty == 0 {
                return Err(Reject::new(
                    RejectCode::InsufficientGoods,
                    format!("{borrower:?} holds {held} shares of {org}"),
                ));
            }
        }
    }
    Ok(())
}

/// Whether an asset is pledged under an open credit contract.
#[must_use]
pub fn pledged(world: &World, collateral: Collateral) -> bool {
    world.contracts.values().any(|k| {
        k.status != ContractStatus::Ended
            && matches!(k.body, ContractBody::Credit { collateral: Some(c), .. } if c == collateral)
    })
}

/// `AcceptCredit`: the borrower takes the loan and pledges the collateral.
pub fn accept_credit(
    world: &World,
    envelope: &Envelope<Command>,
    id: OfferId,
) -> Result<Vec<Event>, Reject> {
    let borrower = acting_party(world, envelope)?;
    let offer = world
        .offers
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOffer, format!("no offer {id}")))?;
    let OfferBody::Credit {
        to,
        principal,
        rate_per_cycle_bp,
        term_cycles,
        collateral,
    } = offer.body
    else {
        return Err(Reject::new(
            RejectCode::UnknownOffer,
            format!("offer {id} is not a loan"),
        ));
    };
    if offer.by == borrower {
        return Err(Reject::new(
            RejectCode::SelfDeal,
            "cannot borrow from yourself",
        ));
    }
    if let Some(t) = to
        && t != borrower
    {
        return Err(Reject::new(
            RejectCode::NotParty,
            format!("offer {id} is addressed to {t:?}"),
        ));
    }
    if let Party::Citizen(c) = borrower
        && world.citizens[&c].flags.options_narrowed
        && term_cycles > world.params.contracts.long_contract_cycles
    {
        return Err(Reject::new(
            RejectCode::OptionsNarrowed,
            "a destitute citizen cannot sign a long loan",
        ));
    }
    if let Some(c) = collateral {
        owns_collateral(world, borrower, c)?;
    }
    let (_, installment, _) = schedule(principal, rate_per_cycle_bp, term_cycles);
    Ok(vec![Event::CreditAccepted {
        contract: world.next.contract,
        lender: offer.by,
        borrower,
        principal,
        rate_per_cycle_bp,
        installment,
        installments: term_cycles,
        collateral,
    }])
}

/// Step 8c: installments, borrower to lender. A borrower who cannot pay pays
/// nothing; the miss resolves as a default in phase 7 of the next tick.
#[allow(clippy::type_complexity)]
pub fn cycle_end_8c_credit_installments(b: &mut TickBuilder) {
    let loans: Vec<(ContractId, Party, Party, Money, u32, Money, u32, u32)> = b
        .world
        .contracts
        .values()
        .filter(|k| k.status == ContractStatus::Active)
        .filter_map(|k| match k.body {
            ContractBody::Credit {
                principal,
                rate_per_cycle_bp,
                installment,
                installments_left,
                ..
            } if installments_left > 0 => Some((
                k.id,
                k.parties.0,
                k.parties.1,
                principal,
                rate_per_cycle_bp,
                installment,
                installments_left,
                k.term_cycles.unwrap_or(installments_left),
            )),
            _ => None,
        })
        .collect();
    for (id, _lender, borrower, principal, rate, installment, left, term) in loans {
        let (_, _, last) = schedule(principal, rate, term);
        let due = if left == 1 { last } else { installment };
        let have = crate::apply::money_of(&b.world, borrower);
        if have >= due {
            let explain = Explain::new(
                RuleId::CreditInstallment,
                "(principal + principal x rate x term) / term",
                due,
            )
            .input("principal", principal)
            .input("rate_per_cycle", f64::from(rate) / 10_000.0)
            .input("term_cycles", term)
            .input("installments_left", left);
            b.emit(Event::CreditInstallment {
                contract: id,
                amount: due,
                remaining: left - 1,
                explain,
            });
            if left == 1 {
                b.emit(Event::CreditRepaid { contract: id });
            }
        } else {
            b.emit(Event::CreditMissed {
                contract: id,
                owed: due,
            });
        }
    }
}

/// Phase 7 (first tick of a cycle): defaults for installments missed at the last cycle end.
pub fn phase_7_defaults(b: &mut TickBuilder) {
    if !b.tick.is_multiple_of(b.world.params.time.ticks_per_cycle) {
        return;
    }
    let defaults: Vec<(ContractId, Option<Collateral>)> = b
        .world
        .contracts
        .values()
        .filter(|k| k.status == ContractStatus::Active)
        .filter_map(|k| match k.body {
            ContractBody::Credit {
                missed: true,
                collateral,
                ..
            } => Some((k.id, collateral)),
            _ => None,
        })
        .collect();
    for (id, collateral) in defaults {
        b.emit(Event::CreditDefaulted {
            contract: id,
            collateral_seized: collateral,
        });
    }
}

// ---------------------------------------------------------------------------
// Associations (GDD §7.1): the catch-all org. Membership by request and
// admission; disbursement by the manager until member votes arrive (Phase 2).

fn association(world: &World, org: OrgId) -> Result<&crate::world::Org, Reject> {
    let o = world
        .orgs
        .get(&org)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {org}")))?;
    if !matches!(
        o.kind,
        OrgKind::Association | OrgKind::Cooperative | OrgKind::Union
    ) {
        return Err(Reject::new(
            RejectCode::NotInThisSociety,
            format!("{} has no members", o.name),
        ));
    }
    Ok(o)
}

/// `RequestMembership`: a standing request on the notice board.
pub fn request_membership(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let o = association(world, org)?;
    if o.members.contains(&citizen.id) {
        return Err(Reject::new(RejectCode::AlreadyExists, "already a member"));
    }
    if world.offers.values().any(|f| matches!(f.body, OfferBody::Membership { org: x, citizen: c } if x == org && c == citizen.id)) {
        return Err(Reject::new(RejectCode::AlreadyExists, "already requested"));
    }
    Ok(vec![Event::MembershipRequested {
        offer: world.next.offer,
        org,
        citizen: citizen.id,
    }])
}

/// `AdmitMember`: the manager admits a citizen (with or without a request).
pub fn admit_member(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
    citizen: crate::ids::CitizenId,
) -> Result<Vec<Event>, Reject> {
    let actor = acting_citizen(world, envelope)?;
    let o = association(world, org)?;
    if o.manager != Some(actor.id) {
        return Err(Reject::new(
            RejectCode::NotManager,
            "only the manager admits (member votes arrive in Phase 2)",
        ));
    }
    if !world.citizens.contains_key(&citizen) {
        return Err(Reject::new(
            RejectCode::UnknownCitizen,
            format!("no citizen {citizen}"),
        ));
    }
    if o.members.contains(&citizen) {
        return Err(Reject::new(RejectCode::AlreadyExists, "already a member"));
    }
    Ok(vec![Event::MemberAdmitted { org, citizen }])
}

/// `LeaveOrg`: a member leaves; a manager who leaves vacates the chair.
pub fn leave_org(
    world: &World,
    envelope: &Envelope<Command>,
    org: OrgId,
) -> Result<Vec<Event>, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    let o = association(world, org)?;
    if !o.members.contains(&citizen.id) {
        return Err(Reject::new(RejectCode::NotParty, "not a member"));
    }
    let mut events = crate::coop::leave_events(world, o, citizen.id);
    if o.manager == Some(citizen.id) {
        events.push(Event::ManagerAppointed { org, citizen: None });
    }
    Ok(events)
}
