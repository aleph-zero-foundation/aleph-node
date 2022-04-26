use aleph_primitives::DEFAULT_UNIT_CREATION_DELAY;
use clap::Parser;
use finality_aleph::UnitCreationDelay;

#[derive(Debug, Parser, Clone)]
pub struct AlephCli {
    #[clap(long)]
    unit_creation_delay: Option<u64>,
}

impl AlephCli {
    pub fn unit_creation_delay(&self) -> UnitCreationDelay {
        UnitCreationDelay(
            self.unit_creation_delay
                .unwrap_or(DEFAULT_UNIT_CREATION_DELAY),
        )
    }
}
