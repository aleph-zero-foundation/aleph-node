//! This module declares an `AlephExecutor` which is either a
//! * `WasmExecutor`, for production and test build (when no local debugging is required)
//! * `NativeElseWasmExecutor` for `runtime-benchmarks` and local debugging builds

use sc_service::Configuration;

#[cfg(not(any(feature = "runtime-benchmarks", feature = "aleph-native-runtime",)))]
pub mod aleph_executor {
    use sc_executor::WasmExecutor;

    use super::Configuration;

    type ExtendHostFunctions = (sp_io::SubstrateHostFunctions,);
    pub type Executor = WasmExecutor<ExtendHostFunctions>;

    pub fn get_executor(config: &Configuration) -> Executor {
        sc_service::new_wasm_executor(config)
    }
}

#[cfg(any(feature = "runtime-benchmarks", feature = "aleph-native-runtime",))]
pub mod aleph_executor {
    use sc_executor::NativeElseWasmExecutor;

    use super::Configuration;

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

    pub type Executor = NativeElseWasmExecutor<ExecutorDispatch>;

    pub fn get_executor(config: &Configuration) -> Executor {
        sc_service::new_native_or_wasm_executor(config)
    }
}
