use thiserror::Error;

#[derive(Debug)]
pub struct JsException {
    pub(crate) msg: String,
    pub(crate) stack: Option<String>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("quickjs failed to initialize context")]
    QuickJsContextInit,
    #[error("quickjs failed to initialize runtime")]
    QuickJsRuntimeInit,
    #[error("unexpected null byte")]
    NulError(#[from] std::ffi::NulError),
    #[error("JavaScript exception")]
    JsException(JsException),
    #[error("utf-8 error")]
    Utf8Error(#[source] std::str::Utf8Error),
}
