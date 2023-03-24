use clap::ValueEnum;
use liminal_ark_relations::{
    environment::{
        CircuitField, Groth16, Marlin, NonUniversalSystem, ProvingSystem, RawKeys, UniversalSystem,
        GM17,
    },
    serialization::serialize,
    CanonicalDeserialize, ConstraintSynthesizer,
};

/// All available non universal proving systems.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, ValueEnum)]
pub enum NonUniversalProvingSystem {
    Groth16,
    Gm17,
}

/// All available universal proving systems.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, ValueEnum)]
pub enum UniversalProvingSystem {
    Marlin,
}

/// Any proving system.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SomeProvingSystem {
    NonUniversal(NonUniversalProvingSystem),
    Universal(UniversalProvingSystem),
}

/// API available only for universal proving systems.
impl UniversalProvingSystem {
    pub fn id(&self) -> String {
        format!("{:?}", self).to_lowercase()
    }

    /// Generates SRS. Returns in serialized version.
    pub fn generate_srs(
        &self,
        num_constraints: usize,
        num_variables: usize,
        degree: usize,
    ) -> Vec<u8> {
        match self {
            UniversalProvingSystem::Marlin => {
                Self::_generate_srs::<Marlin>(num_constraints, num_variables, degree)
            }
        }
    }

    fn _generate_srs<S: UniversalSystem>(
        num_constraints: usize,
        num_variables: usize,
        degree: usize,
    ) -> Vec<u8> {
        let srs = S::generate_srs(num_constraints, num_variables, degree);
        serialize(&srs)
    }

    /// Generates proving and verifying key for `circuit` using `srs`. Returns serialized keys.
    pub fn generate_keys<C: ConstraintSynthesizer<CircuitField>>(
        &self,
        circuit: C,
        srs: Vec<u8>,
    ) -> RawKeys {
        match self {
            UniversalProvingSystem::Marlin => Self::_generate_keys::<_, Marlin>(circuit, srs),
        }
    }

    fn _generate_keys<C: ConstraintSynthesizer<CircuitField>, S: UniversalSystem>(
        circuit: C,
        srs: Vec<u8>,
    ) -> RawKeys {
        let srs =
            <<S as UniversalSystem>::Srs>::deserialize(&*srs).expect("Failed to deserialize srs");
        let (pk, vk) = S::generate_keys(circuit, &srs);
        RawKeys {
            pk: serialize(&pk),
            vk: serialize(&vk),
        }
    }
}

/// API available only for non universal proving systems.
impl NonUniversalProvingSystem {
    pub fn id(&self) -> String {
        format!("{:?}", self).to_lowercase()
    }

    /// Generates proving and verifying key for `circuit`. Returns serialized keys.
    pub fn generate_keys<C: ConstraintSynthesizer<CircuitField>>(&self, circuit: C) -> RawKeys {
        match self {
            NonUniversalProvingSystem::Groth16 => self._generate_keys::<_, Groth16>(circuit),
            NonUniversalProvingSystem::Gm17 => self._generate_keys::<_, GM17>(circuit),
        }
    }

    fn _generate_keys<C: ConstraintSynthesizer<CircuitField>, S: NonUniversalSystem>(
        &self,
        circuit: C,
    ) -> RawKeys {
        let (pk, vk) = S::generate_keys(circuit);
        RawKeys {
            pk: serialize(&pk),
            vk: serialize(&vk),
        }
    }
}

/// Common API for all systems.
impl SomeProvingSystem {
    pub fn id(&self) -> String {
        match self {
            SomeProvingSystem::NonUniversal(s) => s.id(),
            SomeProvingSystem::Universal(s) => s.id(),
        }
    }

    /// Generates proof for `circuit` using proving key `pk`. Returns serialized proof.
    pub fn prove<C: ConstraintSynthesizer<CircuitField>>(
        &self,
        circuit: C,
        pk: Vec<u8>,
    ) -> Vec<u8> {
        use SomeProvingSystem::*;

        match self {
            NonUniversal(NonUniversalProvingSystem::Groth16) => {
                Self::_prove::<_, Groth16>(circuit, pk)
            }
            NonUniversal(NonUniversalProvingSystem::Gm17) => Self::_prove::<_, GM17>(circuit, pk),
            Universal(UniversalProvingSystem::Marlin) => Self::_prove::<_, Marlin>(circuit, pk),
        }
    }

    fn _prove<C: ConstraintSynthesizer<CircuitField>, S: ProvingSystem>(
        circuit: C,
        pk: Vec<u8>,
    ) -> Vec<u8> {
        let pk = <S::ProvingKey>::deserialize(&*pk).expect("Failed to deserialize proving key");
        let proof = S::prove(&pk, circuit);
        serialize(&proof)
    }

    /// Verifies proof.
    pub fn verify(&self, vk: Vec<u8>, proof: Vec<u8>, input: Vec<u8>) -> bool {
        use SomeProvingSystem::*;

        match self {
            NonUniversal(NonUniversalProvingSystem::Groth16) => {
                Self::_verify::<Groth16>(vk, proof, input)
            }
            NonUniversal(NonUniversalProvingSystem::Gm17) => {
                Self::_verify::<GM17>(vk, proof, input)
            }
            Universal(UniversalProvingSystem::Marlin) => Self::_verify::<Marlin>(vk, proof, input),
        }
    }

    fn _verify<S: ProvingSystem>(vk: Vec<u8>, proof: Vec<u8>, input: Vec<u8>) -> bool {
        let vk = <S::VerifyingKey>::deserialize(&*vk).expect("Failed to deserialize verifying key");
        let proof = <S::Proof>::deserialize(&*proof).expect("Failed to deserialize proof");
        let input =
            <Vec<CircuitField>>::deserialize(&*input).expect("Failed to deserialize public input");

        S::verify(&vk, &proof, input)
            .map_err(|_| "Failed to verify proof")
            .unwrap()
    }
}
