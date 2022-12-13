use worthless_js_rt::{Context, Runtime};

fn main() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::new(&rt).unwrap();
    dbg!(ctx.eval("function xxx() { return 42; }\nxxx"));
}
