use sc_network::{ReputationChange as Rep, ReputationChange};

/// Cost scalars to be used when reporting peers.
mod cost {
    pub(crate) const PER_UNDECODABLE_BYTE: i32 = -5;
    pub(crate) const UNKNOWN_VOTER: i32 = -150;
    pub(crate) const BAD_SIGNATURE: i32 = -100;
    pub(crate) const OUT_OF_SCOPE_RESPONSE: i32 = -500;
}

pub trait CostBenefit: 'static {
    fn reputation_change(&self) -> Rep;
}

#[derive(Debug)]
pub(crate) enum PeerMisbehavior {
    UndecodablePacket(i32),
    UnknownVoter,
    BadSignature,
    OutOfScopeResponse,
}

impl PeerMisbehavior {
    pub(crate) fn cost(&self) -> Rep {
        use PeerMisbehavior::*;

        match *self {
            UndecodablePacket(bytes) => Rep::new(
                bytes.saturating_mul(cost::PER_UNDECODABLE_BYTE),
                "Aleph: Bad packet",
            ),
            UnknownVoter => Rep::new(cost::UNKNOWN_VOTER, "Aleph: Unknown voter"),
            BadSignature => Rep::new(cost::BAD_SIGNATURE, "Aleph: Bad signature"),
            OutOfScopeResponse => Rep::new(
                cost::OUT_OF_SCOPE_RESPONSE,
                "Aleph: Out-of-scope response message",
            ),
        }
    }
}

impl CostBenefit for PeerMisbehavior {
    fn reputation_change(&self) -> ReputationChange {
        self.cost()
    }
}

/// Benefit scalars used to report good peers.
mod benefit {
    // NOTE: Not sure if we actually want to give rep for a simple fetch request.
    pub(crate) const GOOD_FETCH_REQUEST: i32 = 0;
    pub(crate) const GOOD_FETCH_RESPONSE: i32 = 100;
    pub(crate) const GOOD_MULTICAST: i32 = 100;
}

#[derive(Debug)]
pub(crate) enum PeerGoodBehavior {
    FetchRequest,
    FetchResponse,
    Multicast,
}

impl PeerGoodBehavior {
    pub(crate) fn benefit(&self) -> Rep {
        use PeerGoodBehavior::*;

        match *self {
            FetchRequest => Rep::new(benefit::GOOD_FETCH_REQUEST, "Aleph: Good fetch request"),
            FetchResponse => Rep::new(benefit::GOOD_FETCH_RESPONSE, "Aleph: Good fetch response"),
            Multicast => Rep::new(benefit::GOOD_MULTICAST, "Aleph: Good multicast message"),
        }
    }
}

impl CostBenefit for PeerGoodBehavior {
    fn reputation_change(&self) -> ReputationChange {
        self.benefit()
    }
}

#[derive(Debug)]
pub(crate) enum Reputation {
    PeerMisbehavior(PeerMisbehavior),
    PeerGoodBehavior(PeerGoodBehavior),
}

impl AsRef<dyn CostBenefit> for Reputation {
    fn as_ref(&self) -> &dyn CostBenefit {
        use Reputation::*;

        match self {
            PeerMisbehavior(m) => m,
            PeerGoodBehavior(g) => g,
        }
    }
}

impl Reputation {
    pub(crate) fn change(&self) -> Rep {
        self.as_ref().reputation_change()
    }
}

impl From<PeerMisbehavior> for Reputation {
    fn from(misbehavior: PeerMisbehavior) -> Self {
        Reputation::PeerMisbehavior(misbehavior)
    }
}

impl From<PeerGoodBehavior> for Reputation {
    fn from(good_behavior: PeerGoodBehavior) -> Self {
        Reputation::PeerGoodBehavior(good_behavior)
    }
}
