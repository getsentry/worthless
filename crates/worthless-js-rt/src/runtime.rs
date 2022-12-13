use std::fmt;
use std::rc::Rc;

use worthless_quickjs_sys::{JSRuntime, JS_FreeRuntime, JS_NewRuntime};

use crate::error::Error;

#[derive(Clone)]
pub struct Runtime {
    handle: Rc<RuntimeHandle>,
}

struct RuntimeHandle {
    ptr: *mut JSRuntime,
}

impl fmt::Debug for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Runtime").finish()
    }
}

impl Runtime {
    /// Creates a new runtime.
    pub fn new() -> Result<Runtime, Error> {
        let ptr = unsafe { JS_NewRuntime() };
        if ptr.is_null() {
            return Err(Error::QuickJsRuntimeInit);
        }

        Ok(Runtime {
            handle: Rc::new(RuntimeHandle { ptr }),
        })
    }

    /// Returns the internal pointer
    pub(crate) fn ptr(&self) -> *mut JSRuntime {
        self.handle.ptr
    }
}

impl Drop for RuntimeHandle {
    fn drop(&mut self) {
        unsafe { JS_FreeRuntime(self.ptr) }
    }
}