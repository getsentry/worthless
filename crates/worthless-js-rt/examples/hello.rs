use worthless_js_rt::{Context, Runtime};

fn main() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::new_primed(&rt).unwrap();
    dbg!(ctx.eval("VERSION"));
}
