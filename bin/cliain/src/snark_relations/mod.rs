use std::path::PathBuf;

use liminal_ark_relations::{serialize, GetPublicInput};
pub use systems::{NonUniversalProvingSystem, SomeProvingSystem, UniversalProvingSystem};

pub use self::relations::RelationArgs;
use crate::snark_relations::io::{
    read_key, read_proof, read_public_input, read_srs, save_keys, save_proving_artifacts, save_srs,
};

mod io;
pub mod parsing;
mod relations;
mod systems;

pub fn generate_srs(
    system: UniversalProvingSystem,
    num_constraints: usize,
    num_variables: usize,
    degree: usize,
) {
    let srs = system.generate_srs(num_constraints, num_variables, degree);
    save_srs(&srs, &system.id());
}

pub fn generate_keys_from_srs(
    relation: RelationArgs,
    system: UniversalProvingSystem,
    srs_file: PathBuf,
) {
    let srs = read_srs(srs_file);
    let keys = system.generate_keys(relation.clone(), srs);
    save_keys(&relation.id(), &system.id(), &keys.pk, &keys.vk);
}

pub fn generate_keys(relation: RelationArgs, system: NonUniversalProvingSystem) {
    let keys = system.generate_keys(relation.clone());
    save_keys(&relation.id(), &system.id(), &keys.pk, &keys.vk);
}

pub fn generate_proof(
    relation: RelationArgs,
    system: SomeProvingSystem,
    proving_key_file: PathBuf,
) {
    let proving_key = read_key(proving_key_file);
    let proof = system.prove(relation.clone(), proving_key);
    let public_input = serialize(&relation.public_input());
    save_proving_artifacts(&relation.id(), &system.id(), &proof, &public_input);
}

pub fn verify(
    verifying_key_file: PathBuf,
    proof_file: PathBuf,
    public_input_file: PathBuf,
    system: SomeProvingSystem,
) -> bool {
    let verifying_key = read_key(verifying_key_file);
    let proof = read_proof(proof_file);
    let public_input = read_public_input(public_input_file);
    system.verify(verifying_key, proof, public_input)
}
