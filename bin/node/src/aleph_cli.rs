use aleph_primitives::DEFAULT_UNIT_CREATION_DELAY;
use finality_aleph::UnitCreationDelay;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
pub struct AlephCli {
    #[structopt(long)]
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
