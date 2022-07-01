use primitives::AuthorityId;

use crate::{
    debug::{element_prompt, entry_prompt, pallet_prompt},
    AnyConnection,
};

pub fn print_storage<C: AnyConnection>(connection: &C) {
    let members: Vec<AuthorityId> = connection
        .as_connection()
        .get_storage_value("Elections", "Members", None)
        .expect("Api call should succeed")
        .expect("Members should always be present");

    println!("{}", pallet_prompt("Elections"));
    println!("{}", entry_prompt("Members"));

    for member in members {
        println!(
            "{}",
            element_prompt(format!("\tMember {:?}", member.to_string()))
        );
    }
}
