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
	cargo build --tests --target wasm32-wasi
	./scripts/run-tests.sh
