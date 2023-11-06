use crate::{
    abft::SignatureSet,
    crypto::Signature,
    data_io::{AlephData, AlephNetworkMessage},
    Hasher,
};

pub type NetworkData =
    current_aleph_bft::NetworkData<Hasher, AlephData, Signature, SignatureSet<Signature>>;

impl AlephNetworkMessage for NetworkData {
    fn included_data(&self) -> Vec<AlephData> {
        self.included_data()
    }
}
