use anyhow::{Error, Result};
use clap::ValueEnum;
use relations::{
    note_from_bytes, FrontendAccount, FrontendMerklePathNode, FrontendMerkleRoot, FrontendNote,
};

use crate::{
    snark_relations::systems::SomeProvingSystem, NonUniversalProvingSystem, UniversalProvingSystem,
};

pub fn parse_frontend_note(frontend_note: &str) -> Result<FrontendNote> {
    Ok(note_from_bytes(frontend_note.as_bytes()))
}

pub fn parse_frontend_merkle_root(frontend_merkle_root: &str) -> Result<FrontendMerkleRoot> {
    Ok(note_from_bytes(frontend_merkle_root.as_bytes()))
}

pub fn parse_frontend_account(frontend_account: &str) -> Result<FrontendAccount> {
    Ok(frontend_account.as_bytes().try_into().unwrap())
}

pub fn parse_frontend_merkle_path_single(
    frontend_merkle_path_single: &str,
) -> Result<FrontendMerklePathNode> {
    Ok(note_from_bytes(frontend_merkle_path_single.as_bytes()))
}

pub fn parse_some_system(system: &str) -> Result<SomeProvingSystem> {
    let maybe_universal =
        UniversalProvingSystem::from_str(system, true).map(SomeProvingSystem::Universal);
    let maybe_non_universal =
        NonUniversalProvingSystem::from_str(system, true).map(SomeProvingSystem::NonUniversal);
    maybe_universal.or(maybe_non_universal).map_err(Error::msg)
}
