# worthless-quickjs-sys

This crate wraps the unsafe QuickJS C API for the use in Rust by using bindgen.  It
requires the WASI-SDK to compile so make sure to run the `make download-all` command
in the root of the repository first.

For the high level binding see [`worthless-js-rt`](../worthless-js-rt).

## Notes on Patches

QuickJS does not directly compile on WASI with the WASI SDK so custom patches
were necessary.  These are maintained in the
[getsentry/quickjs](https://github.com/getsentry/quickjs/) repository in the
[worthless-patches](https://github.com/getsentry/quickjs/tree/worthless-patches)
branch.

## Notes on Bindgen

The functions are exposed via bindgen which is incapable of automatically wrapping
inline functions or CPP defines.  As a result for some APIs manual wrappers
were exposed in `quickjs-api`.  This means that some code is not inlined that
probably should, but given the many different layouts that `JSValue`s can have
in QuickJS I do not dare to port this manually for the time being.
