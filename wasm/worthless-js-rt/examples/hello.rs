use worthless_js_rt::{Context, Runtime};

fn main() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::new(&rt).unwrap();
    let rv = ctx.eval(
        r#"
        console.log("Hello World!", 42, 23);

        [console, console.log]
    "#,
    );
    let _ = dbg!(rv);
    //dbg!(ctx.eval("VERSION"));
}
