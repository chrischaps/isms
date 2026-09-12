//! Transfers, direct sales and wanted ads (GDD §7.2 "Sale (direct)", §7.3;
//! TDD §5.3, §5.8). Transfers are a primitive in every society; direct sales
//! escrow the seller's asset at offer time and settle atomically on accept.

use crate::apply::{goods_of, money_of};
use crate::command::{Command, Envelope, Reject, RejectCode, acting_citizen};
use crate::event::Event;
use crate::ids::OfferId;
use crate::kinds::Good;
use crate::ledger::{Asset, Party};
use crate::money::Money;
use crate::world::{Offer, OfferBody, Price, SaleAsset, World};

/// The party an envelope acts as: the citizen, or an org the citizen manages.
pub fn acting_party(world: &World, envelope: &Envelope<Command>) -> Result<Party, Reject> {
    let citizen = acting_citizen(world, envelope)?;
    match envelope.on_behalf_of {
        None => Ok(Party::Citizen(citizen.id)),
        Some(org) => {
            let o = world
                .orgs
                .get(&org)
                .ok_or_else(|| Reject::new(RejectCode::UnknownOrg, format!("no org {org}")))?;
            if o.manager != Some(citizen.id) {
                return Err(Reject::new(
                    RejectCode::NotManager,
                    format!("{} does not manage {}", citizen.id, o.name),
                ));
            }
            Ok(Party::Org(org))
        }
    }
}

fn party_exists(world: &World, party: Party) -> Result<(), Reject> {
    match party {
        Party::Citizen(c) if world.citizens.contains_key(&c) => Ok(()),
        Party::Citizen(c) => Err(Reject::new(
            RejectCode::UnknownCitizen,
            format!("no citizen {c}"),
        )),
        Party::Org(o) if world.orgs.contains_key(&o) => Ok(()),
        Party::Org(o) => Err(Reject::new(RejectCode::UnknownOrg, format!("no org {o}"))),
    }
}

/// Reject if a citizen's pantry cannot take `qty` more of `good` (Q12). Orgs are uncapped.
pub fn check_pantry_room(world: &World, party: Party, good: Good, qty: u32) -> Result<(), Reject> {
    let Party::Citizen(c) = party else {
        return Ok(());
    };
    if let Some(cap) = world.params.pantry.get(&good) {
        let have = goods_of(world, party, good);
        if have + qty > *cap {
            return Err(Reject::new(
                RejectCode::PantryFull,
                format!("{c} holds {have} {good:?}; the pantry cap is {cap}"),
            ));
        }
    }
    Ok(())
}

/// Reject if `party` cannot give `asset`.
pub fn check_has(world: &World, party: Party, asset: Asset) -> Result<(), Reject> {
    match asset {
        Asset::Money(m) => {
            if m <= Money::ZERO {
                return Err(Reject::new(
                    RejectCode::InvalidQuantity,
                    "amount must be positive",
                ));
            }
            let have = money_of(world, party);
            if have < m {
                return Err(Reject::new(
                    RejectCode::InsufficientFunds,
                    format!("{party:?} has {have}"),
                ));
            }
        }
        Asset::Good(g, q) => {
            if q == 0 {
                return Err(Reject::new(
                    RejectCode::InvalidQuantity,
                    "quantity must be positive",
                ));
            }
            let have = goods_of(world, party, g);
            if have < q {
                return Err(Reject::new(
                    RejectCode::InsufficientGoods,
                    format!("{party:?} has {have} {g:?}"),
                ));
            }
        }
    }
    Ok(())
}

/// `Transfer`: money or goods from the acting party to any citizen or org.
pub fn transfer(
    world: &World,
    envelope: &Envelope<Command>,
    to: Party,
    asset: Asset,
    memo: &str,
) -> Result<Vec<Event>, Reject> {
    let from = acting_party(world, envelope)?;
    party_exists(world, to)?;
    if from == to {
        return Err(Reject::new(
            RejectCode::SelfDeal,
            "cannot transfer to yourself",
        ));
    }
    check_has(world, from, asset)?;
    if let Asset::Good(g, q) = asset {
        check_pantry_room(world, to, g, q)?;
    }
    Ok(vec![Event::Transferred {
        from,
        to,
        asset,
        memo: memo.to_owned(),
    }])
}

/// The escrowable form of a sale asset; shares are checked separately.
fn sale_asset_as_asset(asset: SaleAsset) -> Result<Option<Asset>, Reject> {
    match asset {
        SaleAsset::Good(g, q) => Ok(Some(Asset::Good(g, q))),
        SaleAsset::Shares(..) => Ok(None),
        SaleAsset::Dwelling(_) => Err(Reject::new(
            RejectCode::NotImplemented,
            "dwelling sales arrive in S0.11",
        )),
    }
}

fn price_as_asset(price: Price) -> Asset {
    match price {
        Price::Money(m) => Asset::Money(m),
        Price::Good(g, q) => Asset::Good(g, q),
    }
}

/// `OfferSale`: escrow the asset and list it.
#[allow(clippy::single_match_else)] // the None arm grows per asset kind
pub fn offer_sale(
    world: &World,
    envelope: &Envelope<Command>,
    asset: SaleAsset,
    price: Price,
    to: Option<Party>,
) -> Result<Vec<Event>, Reject> {
    let seller = acting_party(world, envelope)?;
    match sale_asset_as_asset(asset)? {
        Some(escrowed) => check_has(world, seller, escrowed)?,
        None => {
            let SaleAsset::Shares(org, qty) = asset else {
                unreachable!()
            };
            if qty == 0 {
                return Err(Reject::new(
                    RejectCode::InvalidQuantity,
                    "quantity must be positive",
                ));
            }
            let held = crate::shares::shares_held(world, seller, org);
            if held < qty {
                return Err(Reject::new(
                    RejectCode::InsufficientGoods,
                    format!("{seller:?} holds {held} shares of {org}"),
                ));
            }
        }
    }
    match price_as_asset(price) {
        Asset::Money(m) if m <= Money::ZERO => {
            return Err(Reject::new(
                RejectCode::InvalidPrice,
                "price must be positive",
            ));
        }
        Asset::Good(_, 0) => {
            return Err(Reject::new(
                RejectCode::InvalidPrice,
                "price must be positive",
            ));
        }
        _ => {}
    }
    if let Some(t) = to {
        party_exists(world, t)?;
        if t == seller {
            return Err(Reject::new(
                RejectCode::SelfDeal,
                "cannot offer to yourself",
            ));
        }
    }
    Ok(vec![Event::SaleOffered {
        offer: world.next.offer,
        by: seller,
        asset,
        price,
        to,
    }])
}

fn sale_offer(
    world: &World,
    id: OfferId,
) -> Result<(&Offer, SaleAsset, Price, Option<Party>), Reject> {
    let offer = world
        .offers
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOffer, format!("no offer {id}")))?;
    match &offer.body {
        OfferBody::Sale { asset, price, to } => Ok((offer, *asset, *price, *to)),
        _ => Err(Reject::new(
            RejectCode::UnknownOffer,
            format!("offer {id} is not a sale"),
        )),
    }
}

/// `AcceptSale`: pay the price; the escrowed asset settles to the buyer.
pub fn accept_sale(
    world: &World,
    envelope: &Envelope<Command>,
    id: OfferId,
) -> Result<Vec<Event>, Reject> {
    let buyer = acting_party(world, envelope)?;
    let (offer, asset, price, to) = sale_offer(world, id)?;
    if offer.by == buyer {
        return Err(Reject::new(
            RejectCode::SelfDeal,
            "cannot buy your own offer",
        ));
    }
    if let Some(t) = to
        && t != buyer
    {
        return Err(Reject::new(
            RejectCode::NotParty,
            format!("offer {id} is addressed to {t:?}"),
        ));
    }
    let paid = price_as_asset(price);
    check_has(world, buyer, paid)?;
    if let Asset::Good(g, q) = paid {
        check_pantry_room(world, offer.by, g, q)?;
    }
    if let SaleAsset::Good(g, q) = asset {
        check_pantry_room(world, buyer, g, q)?;
    }
    let mut events = vec![Event::SaleAccepted {
        offer: id,
        buyer,
        seller: offer.by,
        asset,
        price,
    }];
    if let SaleAsset::Shares(org, qty) = asset
        && let Some(h) = crate::shares::holder_of(buyer, org)
        && let Some(e) = crate::shares::control_change(world, org, h, qty)
    {
        events.push(e);
    }
    Ok(events)
}

/// `CancelSale`: release the escrow to the seller.
pub fn cancel_sale(
    world: &World,
    envelope: &Envelope<Command>,
    id: OfferId,
) -> Result<Vec<Event>, Reject> {
    let party = acting_party(world, envelope)?;
    let (offer, ..) = sale_offer(world, id)?;
    if offer.by != party && envelope.actor != crate::event::Actor::System {
        return Err(Reject::new(RejectCode::NotParty, "only the seller cancels"));
    }
    Ok(vec![Event::SaleCancelled { offer: id }])
}

/// `PostWanted`: an informational ad with no escrow and no mechanical effect.
pub fn post_wanted(
    world: &World,
    envelope: &Envelope<Command>,
    good: Good,
    qty: u32,
    max_price: Money,
) -> Result<Vec<Event>, Reject> {
    let by = acting_party(world, envelope)?;
    if qty == 0 {
        return Err(Reject::new(
            RejectCode::InvalidQuantity,
            "quantity must be positive",
        ));
    }
    Ok(vec![Event::WantedPosted {
        offer: world.next.offer,
        by,
        good,
        qty,
        max_price,
    }])
}

/// `RemoveWanted`.
pub fn remove_wanted(
    world: &World,
    envelope: &Envelope<Command>,
    id: OfferId,
) -> Result<Vec<Event>, Reject> {
    let party = acting_party(world, envelope)?;
    let offer = world
        .offers
        .get(&id)
        .ok_or_else(|| Reject::new(RejectCode::UnknownOffer, format!("no offer {id}")))?;
    if !matches!(offer.body, OfferBody::Wanted { .. }) {
        return Err(Reject::new(
            RejectCode::UnknownOffer,
            format!("offer {id} is not a wanted ad"),
        ));
    }
    if offer.by != party {
        return Err(Reject::new(RejectCode::NotParty, "only the poster removes"));
    }
    Ok(vec![Event::WantedRemoved { offer: id }])
}
