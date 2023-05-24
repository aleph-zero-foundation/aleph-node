//! `CodeExecutor` specialization which uses natively compiled runtime when the WASM to be
//! executed is equivalent to the natively compiled code.

use sc_executor::NativeElseWasmExecutor;

// Declare an instance of the native executor named `ExecutorDispatch`. Include the wasm binary as the equivalent wasm code.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
    #[cfg(feature = "runtime-benchmarks")]
    type ExtendHostFunctions = (frame_benchmarking::benchmarking::HostFunctions,);
    #[cfg(not(feature = "runtime-benchmarks"))]
    type ExtendHostFunctions = ();

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        aleph_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        aleph_runtime::native_version()
    }
}

pub type AlephExecutor = NativeElseWasmExecutor<ExecutorDispatch>;
