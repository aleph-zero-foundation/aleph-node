use ark_serialize::CanonicalSerialize;

pub fn serialize<T: CanonicalSerialize>(t: &T) -> Vec<u8> {
    let mut bytes = vec![0; t.serialized_size()];
    t.serialize(&mut bytes[..]).expect("Failed to serialize");
    bytes.to_vec()
}
