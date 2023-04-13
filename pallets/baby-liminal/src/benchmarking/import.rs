use frame_benchmarking::Vec;

pub(super) struct Artifacts {
    pub key: Vec<u8>,
    pub proof: Vec<u8>,
    pub input: Vec<u8>,
}

/// Returns `Artifacts` object built from the resources described with `system` and `relation`
/// arguments.
///
/// Example of usage:
/// ```rust, ignore
/// # use pallet_baby_liminal::get_artifacts;
///
/// let Artifacts { key, proof, input } = get_artifacts!(Groth16, LinearEquation);
/// ```
#[macro_export]
macro_rules! get_artifacts {
    ($system:tt, $relation:tt $(,)?) => {{
        let key = $crate::get_artifact!($system, $relation, VerifyingKey);
        let proof = $crate::get_artifact!($system, $relation, Proof);
        let input = $crate::get_artifact!($system, $relation, PublicInput);

        $crate::benchmarking::import::Artifacts { key, proof, input }
    }};
}

/// Returns a resource as `Vec<u8>`. The retrieval is done in compilation time
/// (with `include_bytes!`).
///
/// Since `include_bytes!` accepts only string literals, in order to achieve some kind of brevity,
/// we had to wrap it with light macros (see `system!`, `relation!` and `artifact!` defined in this
/// module).
#[macro_export]
macro_rules! get_artifact {
    ($system:tt, $relation:tt, $artifact:tt $(,)?) => {
        include_bytes!(concat!(
            "../resources/",
            $crate::system!($system),
            "/",
            $crate::relation!($relation),
            ".",
            $crate::artifact!($artifact),
            ".bytes"
        ))
        .to_vec()
    };
}

/// Converts system identifier to a `&static str` that describes corresponding resources directory.
#[macro_export]
macro_rules! system {
    (Groth16) => {
        "groth16"
    };
}

/// Converts relation identifier to a `&static str` that is used in the filename pattern.
#[macro_export]
macro_rules! relation {
    (Xor) => {
        "xor"
    };
    (LinearEquation) => {
        "linear_equation"
    };
    (MerkleTree8) => {
        "merkle_tree_8"
    };
    (MerkleTree64) => {
        "merkle_tree_64"
    };
    (MerkleTree1024) => {
        "merkle_tree_1024"
    };
}

/// Converts artifact identifier to a `&static str` that is used in the filename pattern.
#[macro_export]
macro_rules! artifact {
    (VerifyingKey) => {
        "vk"
    };
    (Proof) => {
        "proof"
    };
    (PublicInput) => {
        "public_input"
    };
}
