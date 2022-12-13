use std::ffi::CString;
use std::fmt;
use std::rc::Rc;

use worthless_quickjs_sys::{
    JSContext, JS_Eval, JS_FreeContext, JS_NewContext, JS_EVAL_TYPE_GLOBAL,
};

use crate::error::Error;
use crate::runtime::Runtime;
use crate::value::Value;

struct ContextHandle {
    ptr: *mut JSContext,
}

#[derive(Clone)]
pub struct Context {
    handle: Rc<ContextHandle>,
    #[allow(unused)]
    rt: Runtime,
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context").finish()
    }
}

impl Context {
    /// Creates a new context.
    pub fn new(rt: &Runtime) -> Result<Context, Error> {
        let ptr = unsafe { JS_NewContext(rt.ptr()) };
        if ptr.is_null() {
            return Err(Error::QuickJsContextInit);
        }

        Ok(Context {
            handle: Rc::new(ContextHandle { ptr }),
            rt: rt.clone(),
        })
    }

    /// Returns a reference to the runtime.
    pub fn rt(&self) -> &Runtime {
        &self.rt
    }

    /// Evaluates some code
    pub fn eval(&self, code: &str) -> Result<Value, Error> {
        let input = CString::new(code)?;
        let script_name = CString::new("<script>")?;
        unsafe {
            Value::from_raw(
                self,
                JS_Eval(
                    self.handle.ptr,
                    input.as_ptr(),
                    code.len() as _,
                    script_name.as_ptr(),
                    JS_EVAL_TYPE_GLOBAL as i32,
                ),
            )
        }
    }

    pub(crate) fn ptr(&self) -> *mut JSContext {
        self.handle.ptr
    }
}

impl Drop for ContextHandle {
    fn drop(&mut self) {
        unsafe {
            JS_FreeContext(self.ptr);
        }
    }
}
