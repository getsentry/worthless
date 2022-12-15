use std::fmt;
use std::rc::Rc;

use worthless_quickjs_sys::{JSRuntime, JS_FreeRuntime, JS_NewRuntime};

use crate::error::Error;

/// Wraps a QuickJS runtime.
///
/// This is a non thread-safe handle like object that can be cloned
/// cheaply to increment the refcount.
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
            return Err(Error::RuntimeInit);
        }

        Ok(Runtime {
            handle: Rc::new(RuntimeHandle { ptr }),
        })
    }

    /// Returns a runtime instance borrowing from a low-level runtime.
    pub(crate) unsafe fn borrow_raw_unchecked(rt: *mut JSRuntime) -> Runtime {
        // leak one refcount so that we don't hit the gc
        let mut handle = Rc::new(RuntimeHandle { ptr: rt });
        std::mem::forget(Rc::clone(&mut handle));
        Runtime { handle }
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
