use std::sync::Arc;

use sc_keystore::LocalKeystore;
use sp_keystore::Keystore as _;

use crate::{
    aleph_primitives::KEY_TYPE,
    crypto::{AuthorityPen, AuthorityVerifier},
    AuthorityId, NodeIndex,
};

pub fn crypto_basics(
    num_crypto_basics: usize,
) -> (Vec<(NodeIndex, AuthorityPen)>, AuthorityVerifier) {
    let keystore = Arc::new(LocalKeystore::in_memory());
    let mut auth_ids = Vec::with_capacity(num_crypto_basics);
    for _ in 0..num_crypto_basics {
        let pk = keystore.ed25519_generate_new(KEY_TYPE, None).unwrap();
        auth_ids.push(AuthorityId::from(pk));
    }
    let mut result = Vec::with_capacity(num_crypto_basics);
    for (i, auth_id) in auth_ids.iter().enumerate() {
        result.push((
            NodeIndex(i),
            AuthorityPen::new(auth_id.clone(), keystore.clone())
                .expect("The keys should sign successfully"),
        ));
    }
    (result, AuthorityVerifier::new(auth_ids))
}
