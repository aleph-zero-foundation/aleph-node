use aleph_primitives::BlockNumber;
use finality_aleph::{AlephJustification, BlockId, Justification, JustificationTranslator};
use futures::channel::mpsc;
use jsonrpsee::{
    core::{error::Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorObject},
};
use sp_runtime::traits::Header;

/// System RPC errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Justification argument is malformatted.
    #[error("{0}")]
    MalformattedJustificationArg(String),
    /// Provided block range couldn't be resolved to a list of blocks.
    #[error("Node is not fully functional: {}", .0)]
    FailedJustificationSend(String),
    /// Justification argument is malformatted.
    #[error("Failed to translate jsutification into an internal one: {}", .0)]
    FailedJustificationTranslation(String),
}

// Base code for all system errors.
const BASE_ERROR: i32 = 2000;
// Justification argument is malformatted.
const MALFORMATTED_JUSTIFICATION_ARG_ERROR: i32 = BASE_ERROR + 1;
// AlephNodeApiServer is failed to send translated justification.
const FAILED_JUSTIFICATION_SEND_ERROR: i32 = BASE_ERROR + 2;
// AlephNodeApiServer failed to translate justification into internal representation.
const FAILED_JUSTIFICATION_TRANSLATION_ERROR: i32 = BASE_ERROR + 3;

impl From<Error> for JsonRpseeError {
    fn from(e: Error) -> Self {
        match e {
            Error::FailedJustificationSend(e) => CallError::Custom(ErrorObject::owned(
                FAILED_JUSTIFICATION_SEND_ERROR,
                e,
                None::<()>,
            )),
            Error::MalformattedJustificationArg(e) => CallError::Custom(ErrorObject::owned(
                MALFORMATTED_JUSTIFICATION_ARG_ERROR,
                e,
                None::<()>,
            )),
            Error::FailedJustificationTranslation(e) => CallError::Custom(ErrorObject::owned(
                FAILED_JUSTIFICATION_TRANSLATION_ERROR,
                e,
                None::<()>,
            )),
        }
        .into()
    }
}

/// Aleph Node RPC API
#[rpc(client, server)]
pub trait AlephNodeApi<Hash, Number> {
    /// Finalize the block with given hash and number using attached signature. Returns the empty string or an error.
    #[method(name = "alephNode_emergencyFinalize")]
    fn aleph_node_emergency_finalize(
        &self,
        justification: Vec<u8>,
        hash: Hash,
        number: Number,
    ) -> RpcResult<()>;
}

/// Aleph Node API implementation
pub struct AlephNode<H, JT>
where
    H: Header<Number = BlockNumber>,
    JT: JustificationTranslator<H> + Send + Sync + Clone + 'static,
{
    import_justification_tx: mpsc::UnboundedSender<Justification<H>>,
    justification_translator: JT,
}

impl<H, JT> AlephNode<H, JT>
where
    H: Header<Number = BlockNumber>,
    JT: JustificationTranslator<H> + Send + Sync + Clone + 'static,
{
    pub fn new(
        import_justification_tx: mpsc::UnboundedSender<Justification<H>>,
        justification_translator: JT,
    ) -> Self {
        AlephNode {
            import_justification_tx,
            justification_translator,
        }
    }
}

impl<H, JT> AlephNodeApiServer<H::Hash, BlockNumber> for AlephNode<H, JT>
where
    H: Header<Number = BlockNumber>,
    JT: JustificationTranslator<H> + Send + Sync + Clone + 'static,
{
    fn aleph_node_emergency_finalize(
        &self,
        justification: Vec<u8>,
        hash: H::Hash,
        number: BlockNumber,
    ) -> RpcResult<()> {
        let justification: AlephJustification =
            AlephJustification::EmergencySignature(justification.try_into().map_err(|_| {
                Error::MalformattedJustificationArg(
                    "Provided justification cannot be converted into correct type".into(),
                )
            })?);
        let justification = self
            .justification_translator
            .translate(justification, BlockId::new(hash, number))
            .map_err(|e| Error::FailedJustificationTranslation(format!("{}", e)))?;
        self.import_justification_tx
            .unbounded_send(justification)
            .map_err(|_| {
                Error::FailedJustificationSend(
                    "AlephNodeApiServer failed to send JustifictionNotification via its channel"
                        .into(),
                )
            })?;
        Ok(())
    }
}
