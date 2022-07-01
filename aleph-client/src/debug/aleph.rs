use primitives::AuthorityId;

use crate::{
    debug::{element_prompt, entry_prompt, pallet_prompt},
    AnyConnection,
};

pub fn print_storage<C: AnyConnection>(connection: &C) {
    let authorities: Vec<AuthorityId> = connection
        .as_connection()
        .get_storage_value("Aleph", "Authorities", None)
        .expect("Api call should succeed")
        .expect("Authorities should always be present");

    println!("{}", pallet_prompt("Aleph"));
    println!("{}", entry_prompt("Authorities"));

    for auth in authorities {
        println!(
            "{}",
            element_prompt(format!("\tAuthority {:?}", auth.to_string()))
        );
    }
}
