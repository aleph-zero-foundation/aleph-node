use crate::{
    debug::{element_prompt, entry_prompt, pallet_prompt},
    Connection,
};
use primitives::AuthorityId;

pub fn print_storage(connection: &Connection) {
    let members: Vec<AuthorityId> = connection
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
