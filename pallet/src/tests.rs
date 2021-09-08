#![cfg(test)]

use crate::mock::*;
use frame_support::assert_ok;

#[test]
fn test_update_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();
        run_session(1);

        Aleph::update_authorities(to_authorities(&[2, 3, 4]).as_slice());

        assert_eq!(Aleph::authorities(), to_authorities(&[2, 3, 4]));
    });
}

#[test]
fn test_initialize_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2]));
    });
}

#[test]
fn test_validators_should_be_none() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        assert_eq!(Aleph::validators(), None);
    });
}

#[test]
fn test_change_validators() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        assert_ok!(Aleph::change_validators(
            Origin::root(),
            vec![AccountId::default()],
            0
        ));

        assert_eq!(Aleph::validators(), Some(vec![AccountId::default()]));
    });
}

#[test]
#[should_panic]
fn fails_to_initialize_again_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        Aleph::initialize_authorities(&to_authorities(&[1, 2, 3]));
    });
}

#[test]
fn test_current_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();

        run_session(1);

        Aleph::update_authorities(to_authorities(&[2, 3, 4]).as_slice());

        assert_eq!(Aleph::authorities(), to_authorities(&[2, 3, 4]));

        run_session(2);

        Aleph::update_authorities(to_authorities(&[1, 2, 3]).as_slice());

        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2, 3]),);
    })
}

#[test]
fn test_next_session_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();

        run_session(1);

        assert_eq!(
            Aleph::next_session_authorities().unwrap(),
            to_authorities(&[1, 2])
        );

        run_session(2);

        assert_eq!(
            Aleph::next_session_authorities().unwrap(),
            to_authorities(&[1, 2])
        );
    })
}
