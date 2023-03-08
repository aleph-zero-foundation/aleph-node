//! `CodeExecutor` specialization which uses natively compiled runtime when the WASM to be
//! executed is equivalent to the natively compiled code.

use sc_executor::NativeElseWasmExecutor;

// Declare an instance of the native executor named `ExecutorDispatch`. Include the wasm binary as the equivalent wasm code.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
    #[cfg(all(feature = "runtime-benchmarks", feature = "liminal"))]
    type ExtendHostFunctions = (
        frame_benchmarking::benchmarking::HostFunctions,
        aleph_primitives::HostFunctions,
    );
    #[cfg(all(feature = "runtime-benchmarks", not(feature = "liminal")))]
    type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;
    #[cfg(all(not(feature = "runtime-benchmarks"), feature = "liminal"))]
    type ExtendHostFunctions = aleph_primitives::HostFunctions;
    #[cfg(all(not(feature = "runtime-benchmarks"), not(feature = "liminal")))]
    type ExtendHostFunctions = ();

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        aleph_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        aleph_runtime::native_version()
    }
}

pub type AlephExecutor = NativeElseWasmExecutor<ExecutorDispatch>;
