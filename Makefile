.PHONY: download-all
download-all: download-wasi-sdk download-binaryen

.PHONY: download-wasi-sdk
download-wasi-sdk:
	./scripts/download-wasi-sdk.sh

.PHONY: download-binaryen
download-binaryen:
	./scripts/download-binaryen.sh
