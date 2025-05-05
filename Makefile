.PHONY: build wit-deps install

default: build

wit-deps:
	wit-deps update

build:
	cargo build --release

install:
	cargo install --path .
