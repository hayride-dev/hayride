.PHONY: build wit-deps

default: build

wit-deps:
	wit-deps update

build:
	cargo build --release

