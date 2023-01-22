use anyhow;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Host error")]
pub enum HostError {
    #[error("WASM module load failed")]
    WasmModuleLoadFailed(#[source] anyhow::Error),
    #[error("WASM module linking failed")]
    WasmModuleLinkingFailed(#[source] anyhow::Error),
    #[error("WASM invocation failed")]
    WasmInvokeFailed(#[source] anyhow::Error),
    #[error("protocol error")]
    ProtocolError(#[source] worthless_bridge::Error),
    #[error("bridge i/o error")]
    BridgeIoError(#[source] std::io::Error),
}
