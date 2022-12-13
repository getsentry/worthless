use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let here = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let wasi_sdk_path = here.join("wasi-sdk");

    if fs::metadata(wasi_sdk_path.join("share/wasi-sysroot")).is_err() {
        panic!("cannot build: wasi-sdk not found, run make download-wasi-sdk in root folder")
    }

    env::set_var("CC", wasi_sdk_path.join("bin/clang"));
    env::set_var("AR", wasi_sdk_path.join("bin/ar"));
    env::set_var(
        "CFLAGS",
        &format!(
            "--sysroot={}",
            wasi_sdk_path.join("share/wasi-sysroot").display()
        ),
    );

    cc::Build::new()
        .files(&[
            "quickjs/cutils.c",
            "quickjs/libbf.c",
            "quickjs/libregexp.c",
            "quickjs/libunicode.c",
            "quickjs/quickjs.c",
        ])
        .define("_GNU_SOURCE", None)
        .define(
            "CONFIG_VERSION",
            format!(
                "\"{}\"",
                fs::read_to_string(here.join("quickjs/VERSION"))
                    .unwrap()
                    .trim()
            )
            .as_str(),
        )
        .define("CONFIG_BIGNUM", None)
        .define("WORTHLESS_PATCHES", None)
        .cargo_metadata(true)
        .debug(true)
        .flag_if_supported("-Wextra")
        .flag_if_supported("-Wno-sign-compare")
        .flag_if_supported("-Wno-missing-field-initializers")
        .flag_if_supported("-Wundef")
        .flag_if_supported("-Wuninitialized")
        .flag_if_supported("-Wunused")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wwrite-strings")
        .flag_if_supported("-Wchar-subscripts")
        .flag_if_supported("-funsigned-char")
        .flag_if_supported("-Wno-implicit-const-int-float-conversion")
        .target("wasm32-wasi")
        .opt_level(2)
        .compile("quickjs");

    let bindings = bindgen::Builder::default()
        .header("quickjs/quickjs.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_args(&[
            "-fvisibility=default",
            &format!("--target={}", env::var("TARGET").unwrap()),
            &format!(
                "--sysroot={}",
                wasi_sdk_path.join("share/wasi-sysroot").display()
            ),
        ])
        .generate()
        .unwrap();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out_dir.join("bindings.rs")).unwrap();
}
