use thiserror::Error;

use crate::value::JsException;

/// Represents an error
#[derive(Error, Debug)]
pub enum Error {
    #[error("quickjs failed to initialize context")]
    ContextInit,
    #[error("quickjs failed to initialize runtime")]
    RuntimeInit,
    #[error("unexpected null byte")]
    NulError(#[from] std::ffi::NulError),
    #[error("JavaScript exception")]
    JsException(JsException),
    #[error("utf-8 error")]
    Utf8Error(#[source] std::str::Utf8Error),
    #[error("int overflow in number conversion")]
    IntOverflow(#[source] std::num::TryFromIntError),
    #[error("length property of object is invalid")]
    InvalidLength,
}
