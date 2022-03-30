// A minimal tool for sending a setCode extrinsic to some node.

use aleph_client::create_connection;
use sp_core::{sr25519, Pair};
use std::fs;
use structopt::StructOpt;
use substrate_api_client::{compose_call, compose_extrinsic, XtStatus};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "send-runtime",
    about = "Send a setCode extrinsic from a Sudo account."
)]
struct Args {
    /// Seed phrase of the Sudo account
    #[structopt(long, short, name = "PHRASE")]
    sudo_phrase: String,

    /// WS address of a node
    #[structopt(long, short, name = "ADDRESS")]
    url: String,

    /// Path to a file with WASM runtime.
    #[structopt(name = "FILE")]
    runtime: String,
}

fn main() {
    let args = Args::from_args();

    let runtime = fs::read(args.runtime).expect("File not found");
    let sudo = keypair_from_string(&args.sudo_phrase);
    let connection = create_connection(&args.url).set_signer(sudo);

    let call = compose_call!(connection.metadata, "System", "set_code", runtime);
    let tx = compose_extrinsic!(connection, "Sudo", "sudo_unchecked_weight", call, 0_u64);

    connection
        .send_extrinsic(tx.hex_encode(), XtStatus::Finalized)
        .expect("Could not send extrinsic");
}

fn keypair_from_string(seed: &str) -> sr25519::Pair {
    sr25519::Pair::from_string(seed, None).expect("Can't create pair from seed value")
}
