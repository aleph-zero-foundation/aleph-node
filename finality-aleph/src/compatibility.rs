use parity_scale_codec::{Decode, Encode};

#[derive(Encode, Eq, Decode, PartialEq, Debug, Copy, Clone)]
pub struct Version(pub u16);

pub trait Versioned {
    const VERSION: Version;
}
