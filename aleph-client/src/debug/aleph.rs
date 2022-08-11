use primitives::AuthorityId;

use crate::{
    debug::{element_prompt, entry_prompt, pallet_prompt},
    AnyConnectionExt,
};

pub fn print_storage<C: AnyConnectionExt>(connection: &C) {
    let authorities: Vec<AuthorityId> = connection.read_storage_value("Aleph", "Authorities");

    println!("{}", pallet_prompt("Aleph"));
    println!("{}", entry_prompt("Authorities"));

    for auth in authorities {
        println!(
            "{}",
            element_prompt(format!("\tAuthority {:?}", auth.to_string()))
        );
    }
}
