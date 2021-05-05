#![cfg(test)]

use crate::mock::*;
use codec::Encode;
use frame_support::{
    sp_runtime::testing::{Digest, DigestItem},
    traits::OnFinalize,
};
use primitives::{AuthoritiesLog, ALEPH_ENGINE_ID};

#[test]
fn test_send_new_authorities_on_change() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        System::initialize(
            &1,
            &Default::default(),
            &Default::default(),
            Default::default(),
        );
        Aleph::new_session(true, to_authorities(&[1, 2, 3]));
        Aleph::on_finalize(1u64);

        let expected_log = AuthoritiesLog::WillChange {
            session_id: 1,
            when: 1u64,
            next_authorities: to_authorities(&[1, 2, 3]),
        };

        let header = System::finalize();
        assert_eq!(
            header.digest,
            Digest {
                logs: vec![DigestItem::Consensus(
                    ALEPH_ENGINE_ID,
                    expected_log.encode(),
                )]
            }
        );
    });
}

#[test]
#[should_panic]
fn fails_to_set_again_new_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        Aleph::initialize_authorities(&to_authorities(&[1, 2, 3]));
    });
}

#[test]
fn new_session_updates_index() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        let session_check = |id| {
            if let Some(session_info) = Aleph::session_info() {
                assert_eq!(session_info.session_id, id);
            } else {
                panic!("session_info should not be `None`");
            }
        };

        session_check(0);
        assert_eq!(Staking::current_era(), Some(0));
        run_session(2);
        session_check(1);
        assert_eq!(Staking::current_era(), Some(1));
    });
}
