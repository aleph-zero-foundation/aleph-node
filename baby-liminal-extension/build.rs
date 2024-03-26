//! This build script is used to generate the SNARK artifacts for the benchmarking purposes.
//!
//! # Why build script?
//!
//! Benchmarks are run within the runtime environment. This means that:
//! - cryptography is less effective there,
//! - we don't have access to no-std funcionalities, which means that it is hard to run halo2 prover there (for proof
//! generation)
//!
//! To overcome these problems, we generate the artifacts during building the crate and then use them in the runtime.
//!
//! # What is generated?
//!
//! For every circuit, we generate the following artifacts:
//! - verification key,
//! - proof,
//! - public input.
//!
//! All of them are saved to corresponding files in the `benchmark-resources` directory as raw bytes.
//!
//! # How to run?
//!
//! You just have to build this crate with the `runtime-benchmarks` feature enabled. This will generate the artifacts
//! and put them in the `benchmark-resources` directory. Changing the build script will trigger the artifacts generation
//! again. On the other hand, changing any other file in this crate will not be considered as a reason for rerunning.
//!
//! Note: if a file for a particular circuit already exists, it will not be generated again. Furthermore, if all files
//! are present, the trusted setup procedure (the heaviest computation) will also be skipped.
//!
//! # What circuits are generated?
//!
//! We provide a generic circuit that can be parametrized with the number of instances and the number of rows. More
//! specifically, `BenchCircuit<INSTANCES, ROW_BLOWUP>` is a circuit that:
//! - has `INSTANCES` instances (public inputs);
//! - has `INSTANCES` advices (private inputs); `i`th advice is a square root of `i`th instance;
//! - has `INSTANCES · ROW_BLOWUP` gates (rows);
//!
//! ## Gates
//!
//! First `INSTANCES` gates are the ones that ensure that the corresponding advice is indeed a square root of the
//! corresponding instance.
//!
//! The rest of the gates are batches of `ROW_BLOWUP - 1` copies of the `i`th gate (`i`th batch corresponds to the `ith`
//! gate).

#[cfg(feature = "runtime-benchmarks")]
use {
    artifacts::generate_artifacts,
    halo2_proofs::{halo2curves::bn256::Bn256, poly::kzg::commitment::ParamsKZG},
    std::{cell::OnceCell, env, fs, path::Path},
};

/// This build script is used only for the runtime benchmarking setup. We don't need to do anything here in other case.
#[cfg(not(feature = "runtime-benchmarks"))]
fn main() {}

#[cfg(feature = "runtime-benchmarks")]
fn main() {
    // We rerun the build script only if this file changes. SNARK artifacts generation doesn't depend on any of the
    // source files.
    println!("cargo:rerun-if-changed=build.rs");

    // We run benchmarks for up to ~4K gates - this is to be changed for the final version. Now, we keep it low for
    // developer convenience.
    const CIRCUIT_MAX_K: u32 = 20;
    // We run a common setup for all generated circuits.
    let params = OnceCell::new();

    let path = |instances, row_blowup, suf| {
        Path::new(&env::current_dir().unwrap())
            .join("benchmark-resources")
            .join(format!("{instances}_{row_blowup}_{suf}"))
    };

    const INSTANCES: &[usize] = &[1, 2, 8, 16, 64, 128];
    const ROW_BLOWUP: &[usize] = &[1, 8, 64, 512, 4096];

    for instances in INSTANCES {
        for row_blowup in ROW_BLOWUP {
            let path = |suf| path(*instances, *row_blowup, suf);
            if [path("vk"), path("proof"), path("input")]
                .into_iter()
                .all(|p| p.exists())
            {
                continue;
            }

            let artifacts = generate_artifacts(
                *instances,
                *row_blowup,
                params.get_or_init(|| {
                    ParamsKZG::<Bn256>::setup(CIRCUIT_MAX_K, ParamsKZG::<Bn256>::mock_rng())
                }),
            );

            fs::write(&path("vk"), artifacts.verification_key).unwrap();
            fs::write(&path("proof"), artifacts.proof).unwrap();
            fs::write(&path("input"), artifacts.public_input).unwrap();
        }
    }
}

/// This module contains the code that is used to generate the SNARK artifacts (proof generation).
#[cfg(feature = "runtime-benchmarks")]
mod artifacts {
    use halo2_proofs::{
        halo2curves::bn256::{Bn256, Fr, G1Affine},
        plonk::{create_proof, keygen_pk, keygen_vk, VerifyingKey},
        poly::{
            commitment::Params,
            kzg::{commitment::ParamsKZG, multiopen::ProverGWC},
        },
        transcript::{Blake2bWrite, Challenge255, TranscriptWriterBuffer},
    };

    use crate::circuit::BenchCircuit;

    pub struct Artifacts {
        pub verification_key: Vec<u8>,
        pub proof: Vec<u8>,
        pub public_input: Vec<u8>,
    }

    /// Run the proof generation for the given circuit and parameters.
    pub fn generate_artifacts(
        instances: usize,
        row_blowup: usize,
        params: &ParamsKZG<Bn256>,
    ) -> Artifacts {
        let circuit = BenchCircuit::natural_numbers(instances, row_blowup);
        let instances = (0..instances)
            .map(|i| Fr::from((i * i) as u64))
            .collect::<Vec<_>>();

        let vk = keygen_vk(params, &circuit).expect("vk should not fail");
        let pk = keygen_pk(params, vk.clone(), &circuit).expect("pk should not fail");

        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
        create_proof::<_, ProverGWC<'_, Bn256>, _, _, _, _>(
            params,
            &pk,
            &[circuit],
            &[&[&instances]],
            ParamsKZG::<Bn256>::mock_rng(),
            &mut transcript,
        )
        .expect("prover should not fail");

        Artifacts {
            verification_key: serialize_vk(vk, params.k()),
            proof: transcript.finalize(),
            public_input: instances.iter().flat_map(|i| i.to_bytes()).collect(),
        }
    }

    /// Serializes the verification key to the raw bytes together with the upperbound on the number of gates (required
    /// by the on-chain verifier).
    fn serialize_vk(vk: VerifyingKey<G1Affine>, k: u32) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend(k.to_le_bytes());
        buffer.extend(vk.to_bytes(halo2_proofs::SerdeFormat::RawBytesUnchecked));
        buffer
    }
}

/// This module defines the circuit for the benchmarking purposes.
#[cfg(feature = "runtime-benchmarks")]
mod circuit {
    use halo2_proofs::{
        circuit::{Layouter, Region, Value},
        halo2curves::bn256::Fr,
        plonk::{Circuit, ConstraintSystem, Error},
        standard_plonk::{StandardPlonk, StandardPlonkConfig},
    };

    #[derive(Default)]
    pub struct BenchCircuit {
        /// The number of instances.
        instances: usize,
        /// The row blowup factor.
        row_blowup: usize,
        /// The roots of the instances (`i`th root is a square root of `i`th instance).
        roots: Vec<Fr>,
    }

    impl BenchCircuit {
        /// Create a circuit with the consecutive natural numbers as advices.
        pub fn natural_numbers(instances: usize, row_blowup: usize) -> Self {
            let roots = (0..instances).map(|i| Fr::from(i as u64)).collect();
            Self {
                instances,
                row_blowup,
                roots,
            }
        }

        /// Assign the `idx`th root to the `a` and `b` advices and `-1` to the `q_ab` fixed value.
        ///
        /// We assume here that this is the first row in the region (offset 0).
        fn neg_root_square(
            &self,
            idx: usize,
            region: &mut Region<Fr>,
            config: &StandardPlonkConfig<Fr>,
        ) -> Result<(), Error> {
            region.assign_advice(|| "root", config.a, 0, || Value::known(self.roots[idx]))?;
            region.assign_advice(|| "root", config.b, 0, || Value::known(self.roots[idx]))?;
            region.assign_fixed(
                || "root selector",
                config.q_ab,
                0,
                || Value::known(-Fr::one()),
            )?;
            Ok(())
        }
    }

    impl Circuit<Fr> for BenchCircuit {
        type Config = <StandardPlonk as Circuit<Fr>>::Config;
        type FloorPlanner = <StandardPlonk as Circuit<Fr>>::FloorPlanner;

        fn without_witnesses(&self) -> Self {
            BenchCircuit::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            StandardPlonk::configure(meta)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            for instance_idx in 0..self.instances {
                // For every instance, we ensure that the corresponding advice is indeed a square root of it.
                layouter.assign_region(
                    || format!("check {instance_idx}-th root"),
                    |mut region| self.neg_root_square(instance_idx, &mut region, &config),
                )?;
            }

            for instance_idx in 0..self.instances {
                // We also do some dummy work to blow up the number of rows.
                for copy in 0..(self.row_blowup - 1) {
                    layouter.assign_region(
                        || {
                            format!(
                                "check {instance_idx}-th root ({}/{})",
                                copy + 1,
                                self.row_blowup
                            )
                        },
                        |mut region| {
                            self.neg_root_square(instance_idx, &mut region, &config)?;

                            region.assign_advice_from_instance(
                                || "copied instance",
                                config.instance,
                                instance_idx,
                                config.c,
                                0,
                            )?;
                            region.assign_fixed(
                                || "copied instance selector",
                                config.q_c,
                                0,
                                || Value::known(Fr::one()),
                            )?;
                            Ok(())
                        },
                    )?;
                }
            }

            Ok(())
        }
    }
}
