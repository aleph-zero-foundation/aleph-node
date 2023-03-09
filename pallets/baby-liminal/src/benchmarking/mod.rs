//! To benchmark pallet on your machine, run:
//! ```shell
//! export NODE_ID=5D34dL5prEUaGNQtPPZ3yN5Y6BnkfXunKXXz6fo7ZJbLwRRH
//!
//! cargo run --release -p aleph-node --features runtime-benchmarks -- bootstrap-chain \
//!     --base-path /tmp/ \
//!     --account-ids $NODE_ID \
//!     --sudo-account-id $NODE_ID \
//!     --chain-id a0snarknet \
//!     --token-symbol SNRKZERO \
//!     --chain-name 'Aleph Zero Snarknet' \
//!     > ./chainspec.json
//!
//! cargo run --release -p aleph-node --features runtime-benchmarks -- benchmark pallet \
//!     --chain=chainspec.json \
//!     --pallet=pallet_baby_liminal \
//!     --extrinsic='*' \
//!     --steps=20 \
//!     --repeat=50 \
//!     --template=.maintain/pallet-weight-template.hbs \
//!     --execution=wasm \
//!     --wasm-execution=compiled \
//!     --output=pallets/baby-liminal/src/weights.rs
//! ```

mod import;
mod suite;
