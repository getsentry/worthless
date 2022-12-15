.PHONY: iterate
iterate:
	cargo build --target wasm32-wasi --example hello
	wasmtime ../../target/wasm32-wasi/debug/examples/hello.wasm

.PHONY: doc
doc:
	cargo doc --target wasm32-wasi

.PHONY: open-doc
open-doc:
	cargo doc --target wasm32-wasi --open

.PHONY: test
test:
	cargo test --target wasm32-wasi -- --nocapture

# Test build of the small hello example into a WASM file
.PHONY: smolbuild
smolbuild:
	cargo build --target wasm32-wasi --release --example hello
	../../vendor/binaryen/bin/wasm-opt -Oz -o hello-smol.wasm ../../target/wasm32-wasi/release/examples/hello.wasm
	ls -alh hello-smol.wasm
