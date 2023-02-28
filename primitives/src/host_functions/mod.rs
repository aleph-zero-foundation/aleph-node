// We use such inconvienient representation since in this form passes
// wasm-native boundary without conversion.
// We could use sth more ergonomic but it would require encoding and decoding.
// It's possible to pass some wrapper around Fr by implementing PassBy
pub type Input = (u64, u64, u64, u64);
pub type Output = (u64, u64, u64, u64);

fn field_element(input: Input) -> liminal_ark_poseidon::Fr {
    let input: [u64; 4] = [input.0, input.1, input.2, input.3];
    liminal_ark_poseidon::Fr::new(liminal_ark_poseidon::BigInteger256::new(input))
}

fn output(field_element: liminal_ark_poseidon::Fr) -> Output {
    let a = field_element.0 .0;
    (a[0], a[1], a[2], a[3])
}

#[sp_runtime_interface::runtime_interface]
pub trait Poseidon {
    fn one_to_one_hash(input: Input) -> Output {
        let hash = liminal_ark_poseidon::hash::one_to_one_hash([field_element(input)]);
        output(hash)
    }

    fn two_to_one_hash(input0: Input, input1: Input) -> Output {
        let hash = liminal_ark_poseidon::hash::two_to_one_hash([
            field_element(input0),
            field_element(input1),
        ]);
        output(hash)
    }

    fn four_to_one_hash(input0: Input, input1: Input, input2: Input, input3: Input) -> Output {
        let hash = liminal_ark_poseidon::hash::four_to_one_hash([
            field_element(input0),
            field_element(input1),
            field_element(input2),
            field_element(input3),
        ]);
        output(hash)
    }
}
