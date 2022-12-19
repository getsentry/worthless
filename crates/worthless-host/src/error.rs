use thiserror::Error;

#[derive(Error, Debug)]
#[error("Host error")]
pub enum HostError {
    #[error("WASM module load failed")]
    WasmModuleLoadFailed(#[source] wasi_common::Error),
    #[error("WASM module linking failed")]
    WasmModuleLinkingFailed(#[source] wasi_common::Error),
    #[error("WASM invocation failed")]
    WasmInvokeFailed(#[source] wasi_common::Error),
    #[error("JSON serialization failed")]
    JsonSerializationFailed(#[source] serde_json::Error),
}
