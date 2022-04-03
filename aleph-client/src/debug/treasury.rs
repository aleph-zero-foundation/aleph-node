use crate::{
    debug::{element_prompt, entry_prompt, pallet_prompt},
    Connection,
};
use log::trace;
use pallet_treasury::{Proposal, ProposalIndex};
use sp_core::crypto::AccountId32;
use substrate_api_client::Balance;

pub fn print_storage(connection: &Connection) {
    let proposal_count: u32 = connection
        .get_storage_value("Treasury", "ProposalCount", None)
        .expect("Api call should succeed")
        .unwrap_or(0);

    let approvals: Vec<ProposalIndex> = connection
        .get_storage_value("Treasury", "Approvals", None)
        .expect("Api call should succeed")
        .unwrap_or_default();

    println!("{}", pallet_prompt("Treasury"));
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
            .get_storage_map("Treasury", "Proposals", x, None)
            .unwrap();

        if let Some(p) = p {
            println!("{}", element_prompt(format!("\tProposalId {}: {:?}", x, p)));
        } else {
            trace!("No proposal with id {:?} in the storage", x)
        }
    }
    println!();
}
