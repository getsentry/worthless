use std::borrow::Cow;
use std::ffi::CString;
use std::fmt;
use std::mem::ManuallyDrop;

use smallvec::SmallVec;
use worthless_quickjs_sys::{
    JSContext, JSValue, JS_Call, JS_DefinePropertyValueStr, JS_DefinePropertyValueUint32,
    JS_GetPropertyStr, JS_GetPropertyUint32, JS_IsArray, JS_IsFunction, JS_NewArray,
    JS_NewCFunction2, JS_NewObject, JS_NewStringLen, JS_ThrowInternalError, JS_ToCStringLen2,
    JS_ToFloat64, JS_ToInt64Ext, WL_JS_DupValue, WL_JS_FreeValue, WL_JS_NewBool, WL_JS_NewFloat64,
    WL_JS_NewInt32, JS_PROP_C_W_E, JS_TAG_BIG_INT, JS_TAG_BOOL, JS_TAG_EXCEPTION, JS_TAG_FIRST,
    JS_TAG_FLOAT64, JS_TAG_INT, JS_TAG_NULL, JS_TAG_STRING, JS_TAG_SYMBOL, JS_TAG_UNDEFINED,
    WL_JS_NULL, WL_JS_TRUE, WL_JS_UNDEFINED,
};

use crate::context::Context;
use crate::error::Error;
use crate::js_exception::JsException;
use crate::primitive::Primitive;

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

/// A wrapper around a value from the JS engine.
pub struct Value {
    // note on JSValue here.  We're assuming that JSValue is 64bit because
    // internally it uses JS_NAN_BOXING when compiling to wasi
    pub(crate) raw: JSValue,
    ctx: Context,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Debug)]
        struct Invalid;

        let kind = self.kind();
        match kind {
            ValueKind::Null => return f.debug_struct("Null").finish(),
            ValueKind::Undefined => return f.debug_struct("Undefined").finish(),
            ValueKind::Object => {
                if self.is_array() {
                    let mut t = f.debug_tuple("Array");
                    for idx in 0..self.len().unwrap_or(0) {
                        match self.get_by_index(idx) {
                            Ok(value) => t.field(&value),
                            Err(_) => t.field(&Invalid),
                        };
                    }
                    return t.finish();
                } else if self.is_function() {
                    if let Ok(name) = self.get_property("name") {
                        if name.kind() != ValueKind::Undefined {
                            return f
                                .debug_tuple("Function")
                                .field(&name.to_string_lossy())
                                .finish();
                        }
                    }
                    return f.debug_struct("Function").finish();
                }
            }
            _ => {}
        };
        if let Some(x) = self.as_primitive() {
            fmt::Debug::fmt(&x, f)
        } else {
            f.debug_struct(&format!("{:?}", kind))
                .field("to_string", &self.to_string_lossy())
                .finish()
        }
    }
}

impl Value {
    /// Constructs a value from a raw JS value.
    ///
    /// If the value indicates an exception, the actual exception value is fetched
    /// from the context and returned as wrapped error.
    pub(crate) unsafe fn from_raw(ctx: &Context, raw: JSValue) -> Result<Value, Error> {
        let val = Value::from_raw_unchecked(ctx, raw);

        if val.kind() == ValueKind::Exception {
            // this value is actually an exception.  In that case try to fetch the exception
            // information form the context and crate an error.
            Err(Error::JsException(JsException::from_raw(ctx)))
        } else {
            Ok(val)
        }
    }

    /// Constructs a value from a raw JS value without exception handling.
    pub(crate) unsafe fn from_raw_unchecked(ctx: &Context, raw: JSValue) -> Value {
        Value {
            raw,
            ctx: ctx.clone(),
        }
    }

    /// Creates a value from a primitive.
    ///
    /// # Panics
    ///
    /// Panics when a symbol primitive was attempted to be converted into a
    /// value which is not supported.
    pub fn from_primitive<'a, I>(ctx: &Context, value: I) -> Value
    where
        I: Into<Primitive<'a>>,
    {
        let value: Primitive = value.into();
        match value {
            Primitive::Undefined => unsafe { Value::from_raw_unchecked(ctx, WL_JS_UNDEFINED) },
            Primitive::Null => unsafe { Value::from_raw_unchecked(ctx, WL_JS_NULL) },
            Primitive::Bool(value) => unsafe {
                Value::from_raw_unchecked(ctx, WL_JS_NewBool(ctx.ptr(), value as i32))
            },
            Primitive::I32(value) => unsafe {
                Value::from_raw_unchecked(ctx, WL_JS_NewInt32(ctx.ptr(), value))
            },
            Primitive::I64(value) => Value::from_primitive(
                ctx,
                if value as i32 as i64 == value {
                    Primitive::I32(value as i32)
                } else {
                    Primitive::F64(value as f64)
                },
            ),
            Primitive::F64(value) => unsafe {
                Value::from_raw_unchecked(ctx, WL_JS_NewFloat64(ctx.ptr(), value))
            },
            Primitive::Str(value) => unsafe {
                Value::from_raw_unchecked(
                    ctx,
                    JS_NewStringLen(
                        ctx.ptr(),
                        value.as_bytes().as_ptr() as *const i8,
                        value.len(),
                    ),
                )
            },
            Primitive::InvalidStr(value) => unsafe {
                Value::from_raw_unchecked(
                    ctx,
                    JS_NewStringLen(
                        ctx.ptr(),
                        value.as_bytes().as_ptr() as *const i8,
                        value.len(),
                    ),
                )
            },
            // TODO: this is not exposed in the API, but it could be
            // created by invoking javascript?
            Primitive::Symbol(_) => panic!("cannot create symbols"),
        }
    }

    /// Creates an array from an iterator.
    pub fn from_iter<I: Iterator<Item = V>, V: IntoValue>(ctx: &Context, iter: I) -> Value {
        let rv = Value::new_array(ctx);
        for item in iter {
            rv.append(item.into_value(ctx)).unwrap();
        }
        rv
    }

    /// This is only safe for zero sized functions.
    pub fn from_func<F: Fn(&Value, &[Value]) -> Result<Value, Error> + 'static>(
        ctx: &Context,
        name: &str,
        f: F,
    ) -> Result<Value, Error> {
        // TODO: maybe there is a way to stash away a closure too
        let _ = f;
        assert_eq!(std::mem::size_of::<F>(), 0, "can only wrap ZST functions");

        unsafe extern "C" fn trampoline<F>(
            raw_ctx: *mut JSContext,
            this_val: JSValue,
            argc: i32,
            argv: *mut JSValue,
        ) -> JSValue
        where
            F: Fn(&Value, &[Value]) -> Result<Value, Error> + 'static,
        {
            // we invoke the function purely based on the fact that it's a known zero type
            let func: F = unsafe { std::mem::zeroed() };

            let ctx = Context::borrow_raw_unchecked(raw_ctx);
            let this_val =
                unsafe { Value::from_raw_unchecked(&ctx, WL_JS_DupValue(raw_ctx, this_val)) };
            let args = (0..argc as usize)
                .map(|idx| unsafe {
                    Value::from_raw_unchecked(&ctx, WL_JS_DupValue(raw_ctx, *argv.add(idx)))
                })
                .collect::<SmallVec<[Value; 8]>>();

            match func(&this_val, &args) {
                Ok(value) => value.into_raw(),
                Err(err) => {
                    let err_msg = err.to_string();
                    let msg = match CString::new(err_msg) {
                        Ok(msg) => msg,
                        Err(err) => CString::new(
                            err.into_vec()
                                .into_iter()
                                .filter(|x| *x != 0)
                                .collect::<Vec<_>>(),
                        )
                        .unwrap(),
                    };
                    unsafe {
                        JS_ThrowInternalError(raw_ctx, "%s\x00".as_ptr() as *const i8, msg.as_ptr())
                    }
                }
            }
        }

        unsafe {
            let func = JS_NewCFunction2(
                ctx.ptr(),
                Some(trampoline::<F>),
                name.as_ptr() as *const i8,
                1, // length
                0, // JS_CFUNC_generic
                0, // magic
            );
            if func == 0 {
                return Err(ctx.last_error());
            }
            Ok(Value::from_raw_unchecked(&ctx, func))
        }
    }

    /// Crates an empty array
    pub fn new_array(ctx: &Context) -> Value {
        unsafe { Value::from_raw_unchecked(ctx, JS_NewArray(ctx.ptr())) }
    }

    /// Crates an empty object
    pub fn new_object(ctx: &Context) -> Value {
        unsafe { Value::from_raw_unchecked(ctx, JS_NewObject(ctx.ptr())) }
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

    /// Maps the value into a rust primitive.
    pub fn as_primitive(&self) -> Option<Primitive<'_>> {
        Some(match self.kind() {
            ValueKind::Undefined => Primitive::Undefined,
            ValueKind::Null => Primitive::Null,
            ValueKind::Number if self.tag() == JS_TAG_INT => Primitive::I32(self.as_i32().unwrap()),
            ValueKind::Number => Primitive::F64(self.as_f64().unwrap_or(f64::NAN)),
            ValueKind::Boolean => Primitive::Bool(self.is_true()),
            ValueKind::String => match self.to_string_lossy() {
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
            // this is needed because some values such as symbols for some
            // reason cannot be converted to strings.
            if ptr == std::ptr::null() {
                return Err(self.ctx.last_error());
            }
            let ptr = ptr as *const u8;
            let len = len as usize;
            let buffer = std::slice::from_raw_parts(ptr, len);
            std::str::from_utf8(buffer).map_err(Error::Utf8Error)
        }
    }

    /// Returns the value as string with lossy unicode recovery.
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        unsafe {
            let mut len: usize = 0;
            let ptr = JS_ToCStringLen2(self.ctx.ptr(), &mut len, self.raw, 0);
            if ptr == std::ptr::null() {
                return Cow::Borrowed("");
            }
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
                let mut pres: f64 = 0.0;
                unsafe {
                    JS_ToFloat64(self.ctx.ptr(), &mut pres, self.raw);
                }
                Some(pres)
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
            ValueKind::Number => {
                if self.raw == 0 {
                    false
                } else {
                    self.as_f64() != Some(0.0)
                }
            }
            ValueKind::Boolean => unsafe { self.raw == WL_JS_TRUE },
            ValueKind::String => self.as_str().map_or(false, |x| !x.is_empty()),
            ValueKind::Symbol | ValueKind::Exception | ValueKind::Object => true,
        }
    }

    /// Looks up a property on the object.
    pub fn get_property(&self, key: &str) -> Result<Value, Error> {
        let cstring_key = CString::new(key)?;
        unsafe {
            // NOTE: no DupValue here because this is already incremented
            // in JS_GetPropertyStr
            let raw = JS_GetPropertyStr(self.ctx.ptr(), self.raw, cstring_key.as_ptr());
            Value::from_raw(&self.ctx, raw)
        }
    }

    /// Sets a property to the object.
    pub fn set_property<I: IntoValue>(&self, key: &str, value: I) -> Result<(), Error> {
        self._set_property(key, value.into_value(&self.ctx))
    }

    fn _set_property(&self, key: &str, value: Value) -> Result<(), Error> {
        let key = CString::new(key)?;
        let rv = unsafe {
            WL_JS_DupValue(self.ctx.ptr(), value.raw);
            JS_DefinePropertyValueStr(
                self.ctx.ptr(),
                self.raw,
                key.as_ptr(),
                value.raw,
                JS_PROP_C_W_E as i32,
            )
        };

        if rv < 0 {
            Err(self.ctx.last_error())
        } else {
            Ok(())
        }
    }

    /// Looks up a property by index (eg: array).
    pub fn get_by_index(&self, idx: usize) -> Result<Value, Error> {
        let idx = u32::try_from(idx).map_err(Error::IntOverflow)?;
        unsafe {
            // NOTE: no DupValue here because this is already incremented
            // in JS_GetPropertyUint32
            let raw = JS_GetPropertyUint32(self.ctx.ptr(), self.raw, idx);
            Value::from_raw(&self.ctx, raw)
        }
    }

    /// Appends a value to the end of an array.
    pub fn append<I: IntoValue>(&self, value: I) -> Result<(), Error> {
        self._append(value.into_value(&self.ctx))
    }

    pub fn _append(&self, value: Value) -> Result<(), Error> {
        let rv = unsafe {
            WL_JS_DupValue(self.ctx.ptr(), value.raw);
            JS_DefinePropertyValueUint32(
                self.ctx.ptr(),
                self.raw,
                self.get_property("length")?
                    .as_i64()
                    .and_then(|x| u32::try_from(x).ok())
                    .ok_or_else(|| Error::InvalidLength)?,
                value.raw,
                JS_PROP_C_W_E as i32,
            )
        };

        if rv < 0 {
            Err(self.ctx.last_error())
        } else {
            Ok(())
        }
    }

    /// Places a value at a certain index.
    pub fn set_by_index<I: IntoValue>(&self, idx: usize, value: I) -> Result<(), Error> {
        self._set_by_index(idx, value.into_value(&self.ctx))
    }

    pub fn _set_by_index(&self, idx: usize, value: Value) -> Result<(), Error> {
        let rv = unsafe {
            WL_JS_DupValue(self.ctx.ptr(), value.raw);
            JS_DefinePropertyValueUint32(
                self.ctx.ptr(),
                self.raw,
                u32::try_from(idx).map_err(|_| Error::InvalidLength)?,
                value.raw,
                JS_PROP_C_W_E as i32,
            )
        };

        if rv < 0 {
            Err(self.ctx.last_error())
        } else {
            Ok(())
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

    /// Calls the object.
    pub fn call(&self, receiver: Value, args: &[Self]) -> Result<Value, Error> {
        let args: SmallVec<[JSValue; 10]> = args.iter().map(|v| v.raw).collect();
        let rv = unsafe {
            JS_Call(
                self.ctx.ptr(),
                self.raw,
                receiver.raw,
                args.len() as i32,
                args.as_slice().as_ptr() as *mut JSValue,
            )
        };
        unsafe { Value::from_raw(&self.ctx, rv) }
    }

    /// Returns the internal tag of the value.
    fn tag(&self) -> i32 {
        // TODO: not happy that this is inlined
        let tag = (self.raw >> 32) as i32;
        if (tag - JS_TAG_FIRST) as u32 >= (JS_TAG_FLOAT64 - JS_TAG_FIRST) as u32 {
            JS_TAG_FLOAT64
        } else {
            tag
        }
    }

    /// Returns the length of the value.
    ///
    /// This basically returns the result of the `length` property on the JS side.
    pub fn len(&self) -> Option<usize> {
        match self.kind() {
            ValueKind::Undefined | ValueKind::Null | ValueKind::Number | ValueKind::Boolean => None,
            _ => self
                .get_property("length")
                .ok()?
                .as_i64()
                .and_then(|x| usize::try_from(x).ok()),
        }
    }

    /// Returns a reference to the context.
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// Interprets the value unsafe as i32
    fn i32_unchecked(&self) -> i32 {
        (self.raw & 0xffffffff) as i32
    }

    /// Downgrades the value into the lower type
    pub fn into_raw(self) -> JSValue {
        // consume the refcount
        ManuallyDrop::new(self).raw
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        unsafe { WL_JS_DupValue(self.ctx.ptr(), self.raw) };
        Self {
            raw: self.raw,
            ctx: self.ctx.clone(),
        }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        unsafe {
            WL_JS_FreeValue(self.ctx.ptr(), self.raw);
        }
    }
}

pub trait IntoValue {
    fn into_value(self, ctx: &Context) -> Value;
}

impl IntoValue for Value {
    fn into_value(self, _ctx: &Context) -> Value {
        self
    }
}

impl<'a, T: Into<Primitive<'a>>> IntoValue for T {
    fn into_value(self, ctx: &Context) -> Value {
        Value::from_primitive(ctx, self)
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::primitive::Primitive;
    use crate::{Context, ValueKind};

    #[test]
    fn test_null() {
        Context::run(|ctx| {
            let val = Value::from_primitive(ctx, Primitive::Null);
            assert_eq!(val.kind(), ValueKind::Null);
            assert_eq!(val.as_primitive(), Some(Primitive::Null));
            assert!(!val.is_true());
            assert_eq!(val.to_string_lossy(), "null");
            assert_eq!(val.len(), None);
            Ok(())
        })
        .unwrap()
    }

    #[test]
    fn test_undefined() {
        Context::run(|ctx| {
            let val = Value::from_primitive(ctx, Primitive::Undefined);
            assert_eq!(val.kind(), ValueKind::Undefined);
            assert_eq!(val.as_primitive(), Some(Primitive::Undefined));
            assert!(!val.is_true());
            assert_eq!(val.to_string_lossy(), "undefined");
            assert_eq!(val.len(), None);
            Ok(())
        })
        .unwrap()
    }

    #[test]
    fn test_bool() {
        Context::run(|ctx| {
            let val = Value::from_primitive(ctx, true);
            assert_eq!(val.kind(), ValueKind::Boolean);
            assert_eq!(val.as_primitive(), Some(Primitive::Bool(true)));
            assert_eq!(val.to_string_lossy(), "true");
            assert_eq!(val.len(), None);

            let val = Value::from_primitive(ctx, false);
            assert_eq!(val.kind(), ValueKind::Boolean);
            assert_eq!(val.as_primitive(), Some(Primitive::Bool(false)));
            assert_eq!(val.to_string_lossy(), "false");
            assert_eq!(val.len(), None);

            Ok(())
        })
        .unwrap()
    }

    #[test]
    fn test_i32() {
        Context::run(|ctx| {
            let val = Value::from_primitive(ctx, 42i32);
            assert_eq!(val.kind(), ValueKind::Number);
            assert_eq!(val.as_primitive(), Some(Primitive::I32(42)));
            assert_eq!(val.to_string_lossy(), "42");
            assert_eq!(val.len(), None);
            Ok(())
        })
        .unwrap()
    }

    #[test]
    fn test_i64() {
        Context::run(|ctx| {
            let val = Value::from_primitive(ctx, 42i64);
            assert_eq!(val.kind(), ValueKind::Number);
            assert_eq!(val.as_primitive(), Some(Primitive::I32(42)));
            assert_eq!(val.to_string_lossy(), "42");
            assert_eq!(val.len(), None);

            let val = Value::from_primitive(ctx, 4244444444444i64);
            assert_eq!(val.kind(), ValueKind::Number);
            assert_eq!(val.as_primitive(), Some(Primitive::F64(4244444444444f64)));
            assert_eq!(val.to_string_lossy(), "4244444444444");
            assert_eq!(val.len(), None);

            Ok(())
        })
        .unwrap()
    }

    #[test]
    fn test_str() {
        Context::run(|ctx| {
            let val = Value::from_primitive(ctx, "Hello World!");
            assert_eq!(val.kind(), ValueKind::String);
            assert_eq!(val.as_primitive(), Some(Primitive::Str("Hello World!")));
            assert_eq!(val.to_string_lossy(), "Hello World!");
            assert_eq!(val.len(), Some(12));

            Ok(())
        })
        .unwrap()
    }

    #[test]
    fn test_array() {
        Context::run(|ctx| {
            let arr = [
                Value::from_primitive(ctx, "Hello"),
                Value::from_primitive(ctx, "World"),
            ];
            let val = Value::from_iter(ctx, (&arr[..]).iter().cloned());
            assert_eq!(val.kind(), ValueKind::Object);
            assert!(val.is_array());
            assert_eq!(val.get_by_index(0).unwrap().to_string_lossy(), "Hello");
            assert_eq!(val.get_by_index(1).unwrap().to_string_lossy(), "World");
            assert_eq!(val.get_by_index(2).unwrap().kind(), ValueKind::Undefined);
            assert_eq!(val.get_property("0").unwrap().to_string_lossy(), "Hello");
            assert_eq!(val.get_property("1").unwrap().to_string_lossy(), "World");
            assert_eq!(val.get_property("2").unwrap().kind(), ValueKind::Undefined);
            assert_eq!(val.as_primitive(), None);
            assert_eq!(val.to_string_lossy(), "Hello,World");
            assert_eq!(val.len(), Some(2));

            Ok(())
        })
        .unwrap()
    }

    #[test]
    fn test_object() {
        Context::run(|ctx| {
            let val = Value::new_object(ctx);
            val.set_property("a", Value::from_primitive(ctx, 42))?;
            val.set_property("b", Value::from_primitive(ctx, 23))?;
            assert_eq!(val.kind(), ValueKind::Object);
            assert_eq!(val.to_string_lossy(), "[object Object]");
            assert_eq!(val.as_primitive(), None);
            assert_eq!(val.get_property("a").unwrap().to_string_lossy(), "42");
            assert_eq!(val.get_property("b").unwrap().to_string_lossy(), "23");
            assert!(!val.is_array());
            Ok(())
        })
        .unwrap();
    }
}
