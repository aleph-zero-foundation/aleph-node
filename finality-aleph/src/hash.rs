use rush::HashT;
use sp_core::{H160, H256, H512};
use sp_runtime::traits::{
    MaybeDisplay, MaybeMallocSizeOf, MaybeSerializeDeserialize, Member, SimpleBitOps,
};

pub trait Hash:
    Member + MaybeSerializeDeserialize + MaybeDisplay + SimpleBitOps + MaybeMallocSizeOf + HashT
{
}

impl Hash for H160 {}

impl Hash for H256 {}

impl Hash for H512 {}
