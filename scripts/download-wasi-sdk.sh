#!/usr/bin/env bash

VERSION_MAJOR="17"
VERSION_MINOR="0"
SDKROOT="vendor/wasi-sdk"
TEMP_DIR="$(mktemp)"
if [[ "$(uname -s)" == "Darwin" ]]; then
  curl -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${VERSION_MAJOR}/wasi-sdk-${VERSION_MAJOR}.${VERSION_MINOR}-macos.tar.gz --output $TEMP_DIR
else
  curl -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${VERSION_MAJOR}/wasi-sdk-${VERSION_MAJOR}.${VERSION_MINOR}-linux.tar.gz --output $TEMP_DIR
fi

mkdir -p $SDKROOT
(cd $SDKROOT; tar xf $TEMP_DIR --strip-components=1)
