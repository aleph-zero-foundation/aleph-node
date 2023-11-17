use crate::{
    abft::SignatureSet,
    block::UnverifiedHeader,
    crypto::Signature,
    data_io::{AlephData, AlephNetworkMessage},
    Hasher,
};

pub type NetworkData<UH> =
    current_aleph_bft::NetworkData<Hasher, AlephData<UH>, Signature, SignatureSet<Signature>>;

impl<UH: UnverifiedHeader> AlephNetworkMessage<UH> for NetworkData<UH> {
    fn included_data(&self) -> Vec<AlephData<UH>> {
        self.included_data()
    }
}
