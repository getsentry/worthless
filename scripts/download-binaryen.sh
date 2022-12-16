#!/usr/bin/env bash
set -eu

VERSION="111"
SDKROOT="vendor/binaryen"
TEMP_DIR="$(mktemp)"
if [[ "$(uname -s)" == "Darwin" ]]; then
  curl -L https://github.com/WebAssembly/binaryen/releases/download/version_${VERSION}/binaryen-version_${VERSION}-arm64-macos.tar.gz --output $TEMP_DIR
else
  curl -L https://github.com/WebAssembly/binaryen/releases/download/version_${VERSION}/binaryen-version_${VERSION}-x86_64-linux.tar.gz --output $TEMP_DIR
fi

mkdir -p $SDKROOT
(cd $SDKROOT; tar xf $TEMP_DIR --strip-components=1)
