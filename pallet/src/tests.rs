#![cfg(test)]

use crate::mock::*;
use primitives::Session;

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
#[should_panic]
fn fails_to_initialize_again_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        Aleph::initialize_authorities(&to_authorities(&[1, 2, 3]));
    });
}

#[test]
fn test_current_session() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();

        run_session(1);

        Aleph::update_authorities(to_authorities(&[2, 3, 4]).as_slice());

        assert_eq!(
            Aleph::current_session(),
            Session {
                session_id: 1,
                authorities: to_authorities(&[2, 3, 4]),
                stop_h: 1,
            }
        );

        run_session(2);

        Aleph::update_authorities(to_authorities(&[1, 2, 3]).as_slice());

        assert_eq!(
            Aleph::current_session(),
            Session {
                session_id: 2,
                authorities: to_authorities(&[1, 2, 3]),
                stop_h: 2,
            }
        );
    })
}

#[test]
fn test_next_session() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();

        run_session(1);

        assert_eq!(
            Aleph::next_session().unwrap(),
            Session {
                session_id: 2,
                authorities: to_authorities(&[1, 2]),
                stop_h: 2,
            }
        );

        run_session(2);

        assert_eq!(
            Aleph::next_session().unwrap(),
            Session {
                session_id: 3,
                authorities: to_authorities(&[1, 2]),
                stop_h: 3,
            }
        );
    })
}
