use log::trace;
use pallet_treasury::{Proposal, ProposalIndex};
use sp_core::crypto::AccountId32;
use substrate_api_client::Balance;

use crate::{
    debug::{element_prompt, entry_prompt, pallet_prompt},
    ReadStorage,
};

const PALLET: &str = "Treasury";

pub fn print_storage<C: ReadStorage>(connection: &C) {
    let proposal_count: u32 = connection.read_storage_value_or_default(PALLET, "ProposalCount");
    let approvals: Vec<ProposalIndex> =
        connection.read_storage_value_or_default(PALLET, "Approvals");

    println!("{}", pallet_prompt(PALLET));
    println!("{}: {}", entry_prompt("ProposalCount"), proposal_count);
    println!();
    println!("{}", entry_prompt("Approvals"));
    for x in approvals {
        println!(
            "{}",
            element_prompt(format!("Proposal id {} was approved ", x))
        );
    }
    println!();
    println!("{}", entry_prompt("Proposals"));
    for x in 0..=proposal_count {
        let p: Option<Proposal<AccountId32, Balance>> = connection
            .read_storage_map(PALLET, "Proposals", x, None)
            .unwrap();

        if let Some(p) = p {
            println!("{}", element_prompt(format!("\tProposalId {}: {:?}", x, p)));
        } else {
            trace!("No proposal with id {:?} in the storage", x)
        }
    }
    println!();
}
