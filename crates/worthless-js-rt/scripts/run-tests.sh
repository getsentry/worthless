#!/bin/bash

IFS=$'\n'

TESTS="$(cargo test -q --target=wasm32-wasi --no-run --message-format=json | jq -r "select(.profile.test == true) | .filenames[]")"

for test in $TESTS; do
  wasmtime run "$test"
done

