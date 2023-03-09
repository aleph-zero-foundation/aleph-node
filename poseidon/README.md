# Liminal Arkworks Poseidon

This package contains two crates: a library exposing arkworks-based Poseidon hashing and a binary for regenerating parameters.
Parameters are generated using https://github.com/penumbra-zone/poseidon377.

## Library usage

The lib crate `liminal-ark-poseidon` provides two modules:
 - [`hash`](src/hash.rs) module that exposes `<x>_to_one_hash` method family for hashing raw field elements
 - [`circuit`](src/circuit.rs) module that exposes `<x>_to_one_hash` method family for hashing circuit field elements;
it is available only under `circuit` feature flag

Currently, `<x>` is one, two and four, i.e. we support 1:1, 2:1 and 4:1 hashing.

Example usage:
```rust
fn hash_outside_circuit(left: Fr, right: Fr) -> Fr {
    liminal_ark_poseidon::hash::two_to_one_hash([left, right])
}

fn hash_in_circuit(
    cs: ConstraintSystemRef<CircuitField>, 
    left: FpVar<CircuitField>,
    right: FpVar<CircuitField>,
) -> Result<FpVar<CircuitField>, SynthesisError> {
    liminal_ark_poseidon::circuit::two_to_one_hash(cs, [left, right])
}
```

## Binary usage

If you need fresh parameters, you can regenerate [parameters module](src/parameters.rs) by running:
```shell
cargo run --release --features paramgen
```

You can also manipulate security level with `SECURITY_LEVEL` env variable.
