.PHONY: all build run test cov fmt lint clean help

all: build

build:
	cargo build

run:
	cargo run

test:
	cargo test

cov:
	rustup run nightly cargo llvm-cov

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets --all-features -- -D warnings

clean:
	cargo clean

help:
	@echo "Available make targets:"
	@echo "  build   - Build the project (cargo build)"
	@echo "  run     - Run the application (cargo run)"
	@echo "  test    - Run unit tests (cargo test)"
	@echo "  cov     - Run code coverage (rustup run nightly cargo llvm-cov)"
	@echo "  fmt     - Check code formatting (cargo fmt)"
	@echo "  lint    - Run clippy lints (cargo clippy)"
	@echo "  clean   - Clean build artifacts (cargo clean)"
