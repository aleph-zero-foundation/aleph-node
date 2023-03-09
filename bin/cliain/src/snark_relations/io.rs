use std::{fs, path::PathBuf};

fn save_bytes(bytes: &[u8], prefix: &str, identifier: &str) {
    let path = format!("{}.{}.bytes", prefix, identifier);
    fs::write(path, bytes).unwrap_or_else(|_| panic!("Failed to save {}", identifier));
}

pub fn save_srs(srs: &[u8], env_id: &str) {
    save_bytes(srs, env_id, "srs");
}

pub fn save_keys(rel_name: &str, env_id: &str, pk: &[u8], vk: &[u8]) {
    let prefix = format!("{}.{}", rel_name, env_id);
    save_bytes(pk, &prefix, "pk");
    save_bytes(vk, &prefix, "vk");
}

pub fn save_proving_artifacts(rel_name: &str, env_id: &str, proof: &[u8], input: &[u8]) {
    let prefix = format!("{}.{}", rel_name, env_id);
    save_bytes(proof, &prefix, "proof");
    save_bytes(input, &prefix, "public_input");
}

pub fn read_srs(srs_file: PathBuf) -> Vec<u8> {
    fs::read(srs_file).expect("Failed to read SRS from the provided path")
}

pub fn read_key(key_file: PathBuf) -> Vec<u8> {
    fs::read(key_file).expect("Failed to read key from the provided path")
}

pub fn read_proof(proof_file: PathBuf) -> Vec<u8> {
    fs::read(proof_file).expect("Failed to read proof from the provided path")
}

pub fn read_public_input(public_input_file: PathBuf) -> Vec<u8> {
    fs::read(public_input_file).expect("Failed to read public key from the provided path")
}
