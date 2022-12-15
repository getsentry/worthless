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

impl From<bool> for Primitive<'static> {
    fn from(value: bool) -> Primitive<'static> {
        Primitive::Bool(value)
    }
}

impl From<usize> for Primitive<'static> {
    fn from(value: usize) -> Primitive<'static> {
        Primitive::F64(value as f64)
    }
}

impl From<i32> for Primitive<'static> {
    fn from(value: i32) -> Primitive<'static> {
        Primitive::I32(value)
    }
}

impl From<i64> for Primitive<'static> {
    fn from(value: i64) -> Primitive<'static> {
        Primitive::I64(value)
    }
}

impl From<f64> for Primitive<'static> {
    fn from(value: f64) -> Primitive<'static> {
        Primitive::F64(value)
    }
}

impl<'a> From<&'a str> for Primitive<'a> {
    fn from(value: &'a str) -> Self {
        Primitive::Str(value)
    }
}
