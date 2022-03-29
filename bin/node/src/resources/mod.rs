pub fn mainnet_chainspec() -> &'static [u8] {
    include_bytes!("mainnet_chainspec.json")
}

pub fn testnet_chainspec() -> &'static [u8] {
    include_bytes!("testnet_chainspec.json")
}
