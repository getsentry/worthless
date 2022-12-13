use std::ffi::{c_void, CString};
use std::fmt;

use worthless_quickjs_sys::{
    JSRefCountHeader, JSValue, JS_GetException, JS_GetPropertyStr, JS_IsError, JS_ToCStringLen2,
    __JS_FreeValue, JS_TAG_BOOL, JS_TAG_EXCEPTION, JS_TAG_FIRST, JS_TAG_FLOAT64, JS_TAG_INT,
    JS_TAG_NULL, JS_TAG_STRING, JS_TAG_SYMBOL, JS_TAG_UNDEFINED,
};

use crate::context::Context;
use crate::error::{Error, JsException};

#[derive(Debug, PartialEq, Eq)]
pub enum ValueKind {
    Undefined,
    Null,
    Number,
    Boolean,
    String,
    Symbol,
    Object,
}

pub struct Value {
    // note on JSValue here.  We're assuming that JSValue is 64bit because
    // internally it uses JS_NAN_BOXING when compiling to wasi
    raw: JSValue,
    ctx: Context,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Value");
        s.field("kind", &self.kind());
        if let Ok(sval) = self.as_str() {
            s.field("as_str", &sval);
        }
        s.finish()
    }
}

impl Value {
    pub(crate) unsafe fn from_raw(ctx: &Context, raw: JSValue) -> Result<Value, Error> {
        let val = Value {
            raw,
            ctx: ctx.clone(),
        };

        // this value is actually an exception
        if val.raw_tag() == JS_TAG_EXCEPTION {
            let exc_val = unsafe { Value::from_raw(ctx, JS_GetException(ctx.ptr())) }?;
            let msg = exc_val.as_str()?;
            let mut stack = None;
            let is_error = unsafe { JS_IsError(ctx.ptr(), exc_val.raw()) } != 0;
            if is_error {
                let stack_value = val.get_property("stack")?;
                if stack_value.kind() != ValueKind::Undefined {
                    stack.replace(stack_value.as_str().map(ToString::to_string)?);
                }
            }

            Err(Error::JsException(JsException {
                msg: msg.to_string(),
                stack,
            }))
        } else {
            Ok(val)
        }
    }

    pub fn as_str(&self) -> Result<&str, Error> {
        unsafe {
            let mut len: usize = 0;
            let ptr = JS_ToCStringLen2(self.ctx.ptr(), &mut len, self.raw, 0);
            let ptr = ptr as *const u8;
            let len = len as usize;
            let buffer = std::slice::from_raw_parts(ptr, len);
            std::str::from_utf8(buffer).map_err(Error::Utf8Error)
        }
    }

    pub fn get_property(&self, key: &str) -> Result<Self, Error> {
        let cstring_key = CString::new(key)?;
        unsafe {
            let raw = JS_GetPropertyStr(self.ctx.ptr(), self.raw, cstring_key.as_ptr());
            Value::from_raw(&self.ctx, raw)
        }
    }

    pub fn kind(&self) -> ValueKind {
        match self.raw_tag() {
            JS_TAG_UNDEFINED => ValueKind::Undefined,
            JS_TAG_NULL => ValueKind::Null,
            JS_TAG_INT | JS_TAG_FLOAT64 => ValueKind::Number,
            JS_TAG_BOOL => ValueKind::Boolean,
            JS_TAG_STRING => ValueKind::String,
            JS_TAG_SYMBOL => ValueKind::Symbol,
            _ => ValueKind::Object,
        }
    }

    fn raw(&self) -> JSValue {
        self.raw
    }

    fn raw_tag(&self) -> i32 {
        (self.raw >> 32) as _
    }

    fn raw_union(&self) -> RawValueUnion {
        unsafe { std::mem::transmute(self.raw) }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        // see JS_VALUE_HAS_REF_COUNT
        if self.raw_tag() as u32 >= JS_TAG_FIRST as u32 {
            unsafe {
                let ptr = self.raw_union().ptr as *mut JSRefCountHeader;
                (*ptr).ref_count -= 1;
                if (*ptr).ref_count <= 0 {
                    __JS_FreeValue(self.ctx.ptr(), self.raw);
                }
            }
        }
    }
}

#[repr(C)]
union RawValueUnion {
    int32: i32,
    float64: f64,
    ptr: *mut c_void,
}
