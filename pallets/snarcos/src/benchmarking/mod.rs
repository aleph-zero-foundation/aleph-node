mod suite;

fn xor_vk() -> &'static [u8] {
    include_bytes!("resources/xor.vk.bytes")
}

fn xor_proof() -> &'static [u8] {
    include_bytes!("resources/xor.proof.bytes")
}

fn xor_input() -> &'static [u8] {
    include_bytes!("resources/xor.public_input.bytes")
}

fn linear_vk() -> &'static [u8] {
    include_bytes!("resources/linear-equation.vk.bytes")
}

fn linear_proof() -> &'static [u8] {
    include_bytes!("resources/linear-equation.proof.bytes")
}

fn linear_input() -> &'static [u8] {
    include_bytes!("resources/linear-equation.public_input.bytes")
}
