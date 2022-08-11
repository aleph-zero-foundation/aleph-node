use primitives::AuthorityId;

use crate::{
    debug::{element_prompt, entry_prompt, pallet_prompt},
    AnyConnectionExt,
};

pub fn print_storage<C: AnyConnectionExt>(connection: &C) {
    let members: Vec<AuthorityId> = connection.read_storage_value("Elections", "Members");

    println!("{}", pallet_prompt("Elections"));
    println!("{}", entry_prompt("Members"));

    for member in members {
        println!(
            "{}",
            element_prompt(format!("\tMember {:?}", member.to_string()))
        );
    }
}
