//! `Capabilities`: what the constitution enables (TDD §5.2). Derived once per
//! epoch from the constitution, the policy (for the monitoring override) and the
//! params (for the published land slots and rate limit). `handle` consults it
//! before any other validation; the API exposes it so the web client can decide
//! which screens and widgets exist (GDD §15).

use crate::constitution::{
    CapitalMode, Compensation, Constitution, Governance, LaborMode, Monitoring, OfficeSpec,
    Pricing, ProposalKindTag, Proposers, Redistribution,
};
use crate::kinds::{ContractKind, OrgKind, WorkplaceKind};
use crate::params::{Params, RateLimit};
use crate::policy::{MonitoringPolicy, Policy};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// The flags are the capabilities themselves; a struct of them is the point.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Capabilities {
    /// A2 != none.
    pub money: bool,
    /// A2 == market.
    pub order_books: bool,
    /// A2 == administered.
    pub administered_prices: bool,
    /// A2 == none.
    pub common_store: bool,
    pub contracts: BTreeSet<ContractKind>,
    pub org_kinds: BTreeSet<OrgKind>,
    pub labor: LaborMode,
    pub pay: Compensation,
    pub capital: CapitalMode,
    pub redistribution: Redistribution,
    /// A7 as overridden by policy (TDD T14).
    pub monitoring: Monitoring,
    /// The σ the engine actually uses for attribution noise.
    pub monitoring_sigma: f64,
    pub governance: Governance,
    /// The proposal kinds the society may open, and who may open them (S2.1).
    pub proposal_kinds: BTreeSet<ProposalKindTag>,
    pub proposers: Proposers,
    pub offices: Vec<OfficeSpec>,
    /// Slots per workplace kind; `None` = unlimited.
    pub land_slots: BTreeMap<WorkplaceKind, Option<u32>>,
    /// Published so a society's "HFT ceiling" is public.
    pub rate_limit: RateLimit,
}

impl Capabilities {
    #[must_use]
    pub fn derive(constitution: &Constitution, policy: &Policy, params: &Params) -> Self {
        let monitoring = match policy.monitoring {
            MonitoringPolicy::Inherit => constitution.monitoring,
            MonitoringPolicy::High => Monitoring::High,
            MonitoringPolicy::Medium => Monitoring::Medium,
            MonitoringPolicy::Low => Monitoring::Low,
        };
        let monitoring_sigma = match monitoring {
            Monitoring::High => params.monitoring.high,
            Monitoring::Medium => params.monitoring.medium,
            Monitoring::Low => params.monitoring.low,
        };
        let land_slots = WorkplaceKind::ALL
            .iter()
            .map(|k| (*k, params.land.get(k).copied()))
            .collect();
        Capabilities {
            money: constitution.has_money(),
            order_books: constitution.pricing == Pricing::Market,
            administered_prices: constitution.pricing == Pricing::Administered,
            common_store: constitution.pricing == Pricing::None,
            contracts: constitution.contracts.clone(),
            org_kinds: constitution.org_kinds.clone(),
            labor: constitution.labor,
            pay: constitution.compensation,
            capital: constitution.capital,
            redistribution: constitution.redistribution,
            monitoring,
            monitoring_sigma,
            governance: constitution.governance,
            proposal_kinds: constitution.proposal_kinds.clone(),
            proposers: constitution.proposers,
            offices: constitution.offices.clone(),
            land_slots,
            rate_limit: params.market.rate_limit,
        }
    }

    #[must_use]
    pub fn allows_contract(&self, kind: ContractKind) -> bool {
        self.contracts.contains(&kind)
    }

    #[must_use]
    pub fn allows_org(&self, kind: OrgKind) -> bool {
        self.org_kinds.contains(&kind)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::many_single_char_names)]
mod tests {
    use super::*;
    use crate::config::load_preset;
    use crate::constitution::{OfficeKind, RecallRule};
    use std::path::Path;

    fn caps(name: &str) -> Capabilities {
        let p = load_preset(Path::new(crate::WORKSPACE_PRESETS_DIR), name).unwrap();
        Capabilities::derive(&p.constitution, &p.policy, &p.params)
    }

    fn set<T: Ord + Copy>(items: &[T]) -> BTreeSet<T> {
        items.iter().copied().collect()
    }

    /// The S0.2 test table from TDD §5.2.
    #[test]
    fn capability_table_matches_tdd() {
        use crate::constitution::ProposalKindTag as T;
        use ContractKind as C;
        use OrgKind as O;

        let f = caps("freeport");
        assert!(f.money && f.order_books && !f.administered_prices && !f.common_store);
        assert_eq!(
            f.contracts,
            set(&[
                C::Employment,
                C::SaleBook,
                C::SaleDirect,
                C::Credit,
                C::Lease,
                C::Share
            ])
        );
        assert_eq!(f.org_kinds, set(&[O::Firm, O::Association]));
        assert!(f.offices.is_empty());
        assert_eq!(f.monitoring_sigma, 0.0);
        assert_eq!(f.governance, Governance::None);
        assert!(f.proposal_kinds.is_empty());

        let c = caps("commune");
        assert!(!c.money && !c.order_books && c.common_store);
        assert_eq!(c.contracts, set(&[C::SaleDirect, C::Pledge]));
        assert_eq!(c.org_kinds, set(&[O::Collective, O::Association]));
        assert_eq!(c.offices.len(), 1);
        let coord = &c.offices[0];
        assert_eq!(coord.kind, OfficeKind::Coordinator);
        assert_eq!(
            (coord.seats, coord.term_cycles, coord.consecutive),
            (3, 5, false)
        );
        assert_eq!(coord.recall, RecallRule::Majority);
        assert_eq!(c.monitoring_sigma, 0.6);
        assert_eq!(c.pay, Compensation::Need);
        assert_eq!(c.labor, LaborMode::Norm);
        assert_eq!(
            c.proposal_kinds,
            set(&[
                T::PolicyChange,
                T::Resolution,
                T::Election,
                T::Recall,
                T::Honor
            ])
        );
        assert_eq!(c.proposers, crate::constitution::Proposers::Anyone);

        let d = caps("directorate");
        assert!(d.money && d.administered_prices && !d.order_books);
        assert_eq!(d.contracts, set(&[C::SaleDirect, C::Lease]));
        assert_eq!(d.org_kinds, set(&[O::StateEnterprise, O::Association]));
        assert_eq!(d.offices[0].kind, OfficeKind::PlanningCommittee);
        assert_eq!((d.offices[0].seats, d.offices[0].term_cycles), (3, 10));
        assert_eq!(d.offices[0].recall, RecallRule::TwoThirds);
        assert_eq!(d.labor, LaborMode::Assigned);
        assert_eq!(d.monitoring_sigma, 0.25);

        let r = caps("republic");
        assert!(r.money && r.order_books);
        assert_eq!(
            r.contracts,
            set(&[
                C::Employment,
                C::SaleBook,
                C::SaleDirect,
                C::Credit,
                C::Lease,
                C::Share,
                C::CollectiveAgreement
            ])
        );
        assert_eq!(r.org_kinds, set(&[O::Firm, O::Union, O::Association]));
        assert_eq!(r.offices[0].kind, OfficeKind::Legislator);
        assert_eq!(r.offices[0].seats, 5);
        assert_eq!(r.redistribution, Redistribution::TaxTransfer);

        let w = caps("commonwealth");
        assert!(w.money && w.order_books);
        assert_eq!(
            w.contracts,
            set(&[
                C::Employment,
                C::SaleBook,
                C::SaleDirect,
                C::Lease,
                C::PublicCredit
            ])
        );
        assert_eq!(w.org_kinds, set(&[O::Cooperative, O::Association]));
        let kinds: Vec<OfficeKind> = w.offices.iter().map(|o| o.kind).collect();
        assert_eq!(kinds, [OfficeKind::Legislator, OfficeKind::BankBoard]);
        assert_eq!(w.capital, CapitalMode::PublicBank);
        assert_eq!(w.pay, Compensation::Share);
    }

    #[test]
    fn land_slots_are_published_with_none_for_unlimited() {
        let f = caps("freeport");
        assert_eq!(f.land_slots[&WorkplaceKind::Farm], Some(8));
        assert_eq!(f.land_slots[&WorkplaceKind::Mine], Some(6));
        assert_eq!(f.land_slots[&WorkplaceKind::Mill], None);
        assert_eq!(f.land_slots.len(), 7);
        assert_eq!(f.rate_limit.per_second, 5);
    }

    #[test]
    fn policy_monitoring_overrides_constitution() {
        let p = load_preset(Path::new(crate::WORKSPACE_PRESETS_DIR), "commune").unwrap();
        let mut policy = p.policy.clone();
        policy.monitoring = MonitoringPolicy::High;
        let c = Capabilities::derive(&p.constitution, &policy, &p.params);
        assert_eq!(c.monitoring, Monitoring::High);
        assert_eq!(c.monitoring_sigma, 0.0);
    }

    #[test]
    fn capabilities_serialize_for_the_api() {
        let f = caps("freeport");
        let json = serde_json::to_string(&f).unwrap();
        let back: Capabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
        assert!(json.contains("\"order_books\":true"));
    }
}
