use worthless_quickjs_sys::JS_GetException;
use worthless_quickjs_sys::JS_IsError;

use crate::context::Context;
use crate::value::{Value, ValueKind};

/// Represents a JavaScript exception.
#[derive(Debug)]
pub struct JsException {
    pub(crate) msg: String,
    pub(crate) stack: Option<String>,
}

impl JsException {
    /// Returns the error message
    pub fn message(&self) -> &str {
        &self.msg
    }

    /// Returns the stringified stack if available
    pub fn stack(&self) -> Option<&str> {
        self.stack.as_deref()
    }
}

impl JsException {
    pub(crate) unsafe fn from_raw(ctx: &Context) -> JsException {
        let exc_val = unsafe { Value::from_raw_unchecked(ctx, JS_GetException(ctx.ptr())) };
        let msg = exc_val.to_string_lossy().to_string();
        let mut stack = None;
        let is_error = unsafe { JS_IsError(ctx.ptr(), exc_val.raw) } != 0;
        if is_error {
            if let Ok(stack_value) = exc_val.get_property("stack") {
                if stack_value.kind() != ValueKind::Undefined {
                    stack.replace(stack_value.to_string_lossy().to_string());
                }
            }
        }

        JsException {
            msg: msg.to_string(),
            stack,
        }
    }
}
