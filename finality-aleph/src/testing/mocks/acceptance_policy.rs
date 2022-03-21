use std::{cell::RefCell, collections::VecDeque};

#[derive(Clone, Debug)]
pub(crate) enum AcceptancePolicy {
    Unavailable,
    AlwaysAccept,
    AlwaysReject,
    FromSequence(RefCell<VecDeque<bool>>),
}

impl AcceptancePolicy {
    pub(crate) fn accepts(&self) -> bool {
        use AcceptancePolicy::*;

        match &self {
            Unavailable => panic!("Policy is unavailable!"),
            AlwaysAccept => true,
            AlwaysReject => false,
            FromSequence(seq) => seq
                .borrow_mut()
                .pop_front()
                .expect("Not enough values provided!"),
        }
    }
}
