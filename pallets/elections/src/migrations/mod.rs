use sp_std::vec::Vec;

pub mod v0_to_v1;
pub mod v1_to_v2;
pub mod v2_to_v3;

type Validators<T> = Vec<<T as frame_system::Config>::AccountId>;
