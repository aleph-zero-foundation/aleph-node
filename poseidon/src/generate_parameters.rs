#![cfg_attr(not(feature = "std"), no_std)]

use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use ark_bls12_381::{Fr, FrParameters};
use ark_ff::{fields::FpParameters, vec};
use liminal_ark_pnbr_poseidon_paramgen::poseidon_build;

fn main() {
    let security_level = match env::var("SECURITY_LEVEL") {
        Ok(level) => match level.as_str() {
            "80" => 80,
            "128" => 128,
            "256" => 256,
            _ => panic!("Unsupported security level. Supported levels: 80, 128, 256"),
        },
        Err(_) => 128,
    };

    // t = arity + 1, so t=2 is a 1:1 hash, t=3 is a 2:1 hash etc
    // see https://spec.filecoin.io/#section-algorithms.crypto.poseidon.filecoins-poseidon-instances for similar specification used by Filecoin
    let t_values = vec![2, 3, 5];

    // Fr => Fp256
    let parameters =
        poseidon_build::compile::<Fr>(security_level, t_values, FrParameters::MODULUS, true);

    let output_directory = PathBuf::from("./src/parameters.rs");

    let mut file =
        BufWriter::new(File::create(output_directory).expect("can't create source file"));

    let header =
        "//! This file was generated using `generate_parameters.rs`, do not edit it manually!\n";
    file.write_all(header.as_bytes())
        .expect("can write header to file");
    let import_vec = "\nuse ark_ff::vec;\n";
    file.write_all(import_vec.as_bytes())
        .expect("can write import vec to file");
    file.write_all(parameters.as_bytes())
        .expect("can write parameters to file");
}
