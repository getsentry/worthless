# worthless-js-rt

This crate wraps QuickJS so that a WASM module can be compiled that is capable of
executing a reasonable subset of ES5 for pipeline scripting.  It's intended to be
used with in the Sentry pipeline but so far the code here is relatively independent
of assumptions about how Sentry works.

## Runtime Strategy

There are two strategies here about the script.  One is that the WASM runtime just
loads some JavaScript code and reuses the runtime between invocations, the other is
to have Sentry actually "pre-initialize" a JS loaded WASM module with
[wizer](https://crates.io/crates/wizer).
