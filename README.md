# Worthless

This is an experimental WASM based system to build a basic JavaScript execution
system.  Longer term the desire is to enable non JavaScript execution as well, but
for now the runtime environment only permits basic JS execution.

**Current status: this is an experimental project to establish the viability of
WASM module execution in the Sentry pipeline.**

## Contents

Host side:

* [`worthless-host`](crates/worthless-host): this module contains the host side of the
  equation.  It lets one load a WASM module and interact with it.

Guest side:

* [`worthless-quickjs-sys`](wasm/worthless-quickjs-sys): this is a Rust crate that upon
  compilation exposes the unsafe QuickJS API in a WASI compatible build.
* [`worthless-js-rt`](wasm/worthless-js-rt): this is a high level WASI compatible JS
  runtime environment based on quickjs

## Building

For anything here to work you need to some tools to be available.  You can get them
by running `make download-all` in the root folder.

Additionally you need to have `wasmtime` installed on your machine which you can get
with `make install-wasmtime`.

## The Name

Never set your expectations too high.
