use std::ffi::CString;
use std::fmt;
use std::rc::Rc;

use worthless_quickjs_sys::{
    JSContext, JS_Eval, JS_FreeContext, JS_GetGlobalObject, JS_GetRuntime, JS_NewContext,
    JS_EVAL_TYPE_GLOBAL,
};

use crate::builtins::make_basic_console;
use crate::error::Error;
use crate::js_exception::JsException;
use crate::runtime::Runtime;
use crate::value::Value;

struct ContextHandle {
    ptr: *mut JSContext,
}

/// Wraps a QuickJS context.
///
/// This is a non thread-safe handle like object that can be cloned
/// cheaply to increment the refcount.
#[derive(Clone)]
pub struct Context {
    handle: Rc<ContextHandle>,
    rt: Runtime,
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context").finish()
    }
}

impl Context {
    /// Creates a completely empty context.
    pub fn empty(rt: &Runtime) -> Result<Context, Error> {
        let ptr = unsafe { JS_NewContext(rt.as_raw()) };
        if ptr.is_null() {
            return Err(Error::ContextInit);
        }

        Ok(Context {
            handle: Rc::new(ContextHandle { ptr }),
            rt: rt.clone(),
        })
    }

    /// Creates a context populated with common utilities.
    pub fn new(rt: &Runtime) -> Result<Context, Error> {
        let ctx = Context::empty(rt)?;
        let global = ctx.global();
        global.set_property("console", make_basic_console(&ctx)?)?;
        Ok(ctx)
    }

    pub unsafe fn borrow_raw_unchecked(ctx: *mut JSContext) -> Context {
        unsafe {
            let rt_raw = JS_GetRuntime(ctx);
            let rt = Runtime::borrow_raw_unchecked(rt_raw);
            // leak one refcount so that we don't hit the gc
            let mut handle = Rc::new(ContextHandle { ptr: ctx });
            std::mem::forget(Rc::clone(&mut handle));
            Context { handle, rt }
        }
    }

    /// Invokes a function with a new runtime and context.
    pub fn run<R, F>(f: F) -> Result<R, Error>
    where
        F: FnOnce(&Context) -> Result<R, Error>,
    {
        let rt = Runtime::new()?;
        let ctx = Context::new(&rt)?;
        f(&ctx).map_err(Into::into)
    }

    /// Returns a reference to the runtime.
    pub fn rt(&self) -> &Runtime {
        &self.rt
    }

    /// Returns a reference to the root object.
    pub fn global(&self) -> Value {
        // note: inside JS_GetGlobalObject the engine already performs a Js_DupValue
        // so we do not need to do this here.
        unsafe { Value::from_raw_unchecked(self, JS_GetGlobalObject(self.as_raw())) }
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

    /// Returns the last error.
    pub(crate) fn last_error(&self) -> Error {
        Error::JsException(unsafe { JsException::from_raw(self) })
    }

    pub(crate) fn as_raw(&self) -> *mut JSContext {
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
