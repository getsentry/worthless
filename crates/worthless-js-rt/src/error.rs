use thiserror::Error;

/// Represents a JavaScript exception.
#[derive(Debug)]
pub struct JsException {
    pub(crate) msg: String,
    pub(crate) stack: Option<String>,
}

/// Represents an error
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
    #[error("int overflow in number conversion")]
    IntOverflow(#[source] std::num::TryFromIntError),
}
