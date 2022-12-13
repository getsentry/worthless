use std::borrow::Cow;
use std::ffi::{c_void, CString};
use std::fmt;

use worthless_quickjs_sys::{
    JSRefCountHeader, JSValue, JS_GetException, JS_GetPropertyStr, JS_IsArray, JS_IsError,
    JS_IsFunction, JS_ToCStringLen2, JS_ToInt64Ext, __JS_FreeValue, JS_TAG_BIG_INT, JS_TAG_BOOL,
    JS_TAG_EXCEPTION, JS_TAG_FIRST, JS_TAG_FLOAT64, JS_TAG_INT, JS_TAG_NULL, JS_TAG_STRING,
    JS_TAG_SYMBOL, JS_TAG_UNDEFINED,
};

use crate::context::Context;
use crate::error::{Error, JsException};

/// An enum that indicates of what type a value is
#[derive(Debug, PartialEq, Eq)]
pub enum ValueKind {
    Undefined,
    Null,
    Number,
    Boolean,
    String,
    Symbol,
    Exception,
    Object,
}

/// Alternative value representation on the Rust side.
#[derive(Debug, PartialEq)]
pub enum Primitive<'a> {
    Undefined,
    Null,
    Bool(bool),
    I32(i32),
    I64(i64),
    F64(f64),
    Str(&'a str),
    InvalidStr(String),
    Symbol(&'a str),
}

/// A wrapper around a value from the JS engine.
pub struct Value {
    // note on JSValue here.  We're assuming that JSValue is 64bit because
    // internally it uses JS_NAN_BOXING when compiling to wasi
    raw: JSValue,
    ctx: Context,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = self.kind();
        let mut s = if kind == ValueKind::Object {
            if self.is_array() {
                f.debug_struct("Array")
            } else if self.is_function() {
                let mut s = f.debug_struct("Function");
                if let Ok(name) = self.get_property("name") {
                    if name.kind() != ValueKind::Undefined {
                        s.field("name", &name.as_str_lossy());
                    }
                }
                s
            } else {
                f.debug_struct(&format!("{:?}", kind))
            }
        } else {
            f.debug_struct(&format!("{:?}", kind))
        };
        s.field("as_primitive", &self.as_primitive())
            .field("to_string", &self.as_str_lossy())
            .finish()
    }
}

impl Value {
    /// Constructs a value from a raw JS value.
    ///
    /// If the value indicates an exception, the actual exception value is fetched
    /// from the context and returned as wrapped error.
    pub(crate) unsafe fn from_raw(ctx: &Context, raw: JSValue) -> Result<Value, Error> {
        let val = Value::from_raw_unchecked(ctx, raw);

        if val.kind() != ValueKind::Exception {
            return Ok(val);
        }

        // this value is actually an exception.  In that case try to fetch the exception
        // information form the context and crate an error.
        let exc_val = unsafe { Value::from_raw_unchecked(ctx, JS_GetException(ctx.ptr())) };
        let msg = exc_val.as_str()?;
        let mut stack = None;
        let is_error = unsafe { JS_IsError(ctx.ptr(), exc_val.raw) } != 0;
        if is_error {
            if let Ok(stack_value) = exc_val.get_property("stack") {
                if stack_value.kind() != ValueKind::Undefined {
                    stack.replace(stack_value.as_str().map(ToString::to_string)?);
                }
            }
        }

        Err(Error::JsException(JsException {
            msg: msg.to_string(),
            stack,
        }))
    }

    /// Constructs a value from a raw JS value without exception handling.
    unsafe fn from_raw_unchecked(ctx: &Context, raw: JSValue) -> Value {
        Value {
            raw,
            ctx: ctx.clone(),
        }
    }

    /// Maps the value into a rust primitive.
    pub fn as_primitive(&self) -> Option<Primitive<'_>> {
        Some(match self.kind() {
            ValueKind::Undefined => Primitive::Undefined,
            ValueKind::Null => Primitive::Null,
            ValueKind::Number if self.tag() == JS_TAG_INT => Primitive::I32(self.as_i32().unwrap()),
            ValueKind::Number => Primitive::F64(self.as_f64().unwrap_or(f64::NAN)),
            ValueKind::Boolean => Primitive::Bool(self.is_true()),
            ValueKind::String => match self.as_str_lossy() {
                Cow::Borrowed(val) => Primitive::Str(val),
                Cow::Owned(invalid_str) => Primitive::InvalidStr(invalid_str),
            },
            ValueKind::Symbol => match self.as_str() {
                Ok(val) => Primitive::Symbol(val),
                Err(_) => return None,
            },
            ValueKind::Exception | ValueKind::Object => return None,
        })
    }

    /// Returns the value as string.
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

    /// Returns the value as string with lossy unicode recovery.
    pub fn as_str_lossy(&self) -> Cow<'_, str> {
        unsafe {
            let mut len: usize = 0;
            let ptr = JS_ToCStringLen2(self.ctx.ptr(), &mut len, self.raw, 0);
            let ptr = ptr as *const u8;
            let len = len as usize;
            let buffer = std::slice::from_raw_parts(ptr, len);
            String::from_utf8_lossy(buffer)
        }
    }

    /// If the value is a float, returns it.
    pub fn as_f64(&self) -> Option<f64> {
        match self.tag() {
            JS_TAG_FLOAT64 => {
                #[repr(C)]
                union Helper {
                    v: JSValue,
                    d: f64,
                }
                let mut u: Helper = unsafe { std::mem::zeroed() };
                u.v = self
                    .raw
                    .wrapping_add(((0x7ff80000i32.wrapping_sub(JS_TAG_FIRST + 1)) as u64) << 32);
                Some(unsafe { u.d })
            }
            JS_TAG_BIG_INT => {
                let mut pres: i64 = 0;
                unsafe { JS_ToInt64Ext(self.ctx.ptr(), &mut pres, self.raw) };
                if pres as f64 as i64 == pres {
                    Some(pres as f64)
                } else {
                    None
                }
            }
            JS_TAG_INT => Some(self.i32_unchecked() as f64),
            _ => None,
        }
    }

    /// If the value is an integer, returns it.
    pub fn as_i32(&self) -> Option<i32> {
        match self.tag() {
            JS_TAG_FLOAT64 => self.as_f64().and_then(|val| {
                if val as i32 as f64 == val {
                    Some(val as i32)
                } else {
                    None
                }
            }),
            JS_TAG_INT => Some(self.i32_unchecked()),
            JS_TAG_BIG_INT => {
                let mut pres: i64 = 0;
                unsafe { JS_ToInt64Ext(self.ctx.ptr(), &mut pres, self.raw) };
                if pres as i32 as i64 == pres {
                    Some(pres as i32)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Returns the value as i64
    pub fn as_i64(&self) -> Option<i64> {
        if self.tag() == JS_TAG_BIG_INT {
            let mut pres: i64 = 0;
            unsafe { JS_ToInt64Ext(self.ctx.ptr(), &mut pres, self.raw) };
            Some(pres)
        } else {
            self.as_i32().map(Into::into)
        }
    }

    /// Returns `true` if this value is truthy.
    pub fn is_true(&self) -> bool {
        match self.kind() {
            ValueKind::Undefined | ValueKind::Null => false,
            // TODO: this does not handle floats and bigints correctly
            ValueKind::Number => self.raw != 0,
            ValueKind::Boolean => self.raw != 0,
            ValueKind::String => self.as_str().map_or(false, |x| !x.is_empty()),
            ValueKind::Symbol | ValueKind::Exception | ValueKind::Object => true,
        }
    }

    /// Looks up a property on the object.
    pub fn get_property(&self, key: &str) -> Result<Self, Error> {
        let cstring_key = CString::new(key)?;
        unsafe {
            let raw = JS_GetPropertyStr(self.ctx.ptr(), self.raw, cstring_key.as_ptr());
            Value::from_raw(&self.ctx, raw)
        }
    }

    /// Returns the kind of value.
    pub fn kind(&self) -> ValueKind {
        match self.tag() {
            JS_TAG_UNDEFINED => ValueKind::Undefined,
            JS_TAG_NULL => ValueKind::Null,
            JS_TAG_INT | JS_TAG_FLOAT64 => ValueKind::Number,
            JS_TAG_BOOL => ValueKind::Boolean,
            JS_TAG_STRING => ValueKind::String,
            JS_TAG_SYMBOL => ValueKind::Symbol,
            JS_TAG_EXCEPTION => ValueKind::Exception,
            _ => ValueKind::Object,
        }
    }

    /// Checks if this object is a function.
    pub fn is_function(&self) -> bool {
        unsafe { JS_IsFunction(self.ctx.ptr(), self.raw) != 0 }
    }

    /// Checks if this object is an array
    pub fn is_array(&self) -> bool {
        unsafe { JS_IsArray(self.ctx.ptr(), self.raw) == 1 }
    }

    /// Returns the internal tag of the value.
    fn tag(&self) -> i32 {
        let tag = (self.raw >> 32) as i32;
        if (tag - JS_TAG_FIRST) as u32 >= (JS_TAG_FLOAT64 - JS_TAG_FIRST) as u32 {
            JS_TAG_FLOAT64
        } else {
            tag
        }
    }

    /// Interprets the value unsafe as i32
    fn i32_unchecked(&self) -> i32 {
        (self.raw & 0xffffffff) as i32
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        // see JS_VALUE_HAS_REF_COUNT
        if (self.raw >> 32) as u32 >= JS_TAG_FIRST as u32 {
            unsafe {
                let ptr = self.raw as *mut c_void as *mut JSRefCountHeader;
                (*ptr).ref_count -= 1;
                if (*ptr).ref_count <= 0 {
                    __JS_FreeValue(self.ctx.ptr(), self.raw);
                }
            }
        }
    }
}
