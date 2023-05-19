use parity_scale_codec::{Decode, Encode, Error, Input, Output};

use crate::{network::Data, SessionId};

/// Data inside session, sent to validator network.
/// Wrapper for data send over network. We need it to ensure compatibility.
/// The order of the data and session_id is fixed in encode and the decode expects it to be data, session_id.
/// Since data is versioned, i.e. it's encoding starts with a version number in the standardized way,
/// this will allow us to retrofit versioning here if we ever need to change this structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataInSession<D: Data> {
    pub data: D,
    pub session_id: SessionId,
}

impl<D: Data> Decode for DataInSession<D> {
    fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
        let data = D::decode(input)?;
        let session_id = SessionId::decode(input)?;

        Ok(Self { data, session_id })
    }
}

impl<D: Data> Encode for DataInSession<D> {
    fn size_hint(&self) -> usize {
        self.data.size_hint() + self.session_id.size_hint()
    }

    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        self.data.encode_to(dest);
        self.session_id.encode_to(dest);
    }
}
