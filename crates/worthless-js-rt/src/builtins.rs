use crate::context::Context;
use crate::error::Error;
use crate::value::Value;
use crate::Primitive;

pub fn make_basic_console(ctx: &Context) -> Result<Value, Error> {
    let rv = Value::new_object(ctx);
    rv.set_property("log", Value::from_func(ctx, "log", log)?)?;
    Ok(rv)
}

fn log(ctx: &Context, _this: &Value, args: &[Value]) -> Result<Value, Error> {
    let mut buf = String::new();
    for (idx, arg) in args.iter().enumerate() {
        if idx > 0 {
            buf.push(' ');
        }
        buf.push_str(&arg.to_string_lossy());
    }
    eprintln!("[console] {}", buf);
    Ok(Value::from_primitive(ctx, Primitive::Undefined))
}
