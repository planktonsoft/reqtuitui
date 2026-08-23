.PHONY: all build run test cov crap mutants fmt lint pre-commit install-hooks setup-tools clean help

all: build

build:
	cargo build

run:
	cargo run

test:
	cargo test

cov:
	rustup run nightly cargo llvm-cov

crap:
	cargo llvm-cov --workspace --lcov --output-path lcov.info
	cargo crap --workspace --lcov lcov.info --threshold 8 --fail-above

mutants:
	cargo mutants

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets --all-features -- -D warnings

pre-commit: fmt

install-hooks:
	@mkdir -p .git/hooks
	@echo "#!/usr/bin/env sh" > .git/hooks/pre-commit
	@echo "set -e" >> .git/hooks/pre-commit
	@echo "echo '==> Running pre-commit checks...'" >> .git/hooks/pre-commit
	@echo "make pre-commit" >> .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Git pre-commit hook installed successfully."

setup-tools:
	rustup component add llvm-tools-preview
	cargo install --locked cargo-llvm-cov cargo-crap cargo-mutants

clean:
	cargo clean

help:
	@echo "Available make targets:"
	@echo "  build         - Build the project (cargo build)"
	@echo "  run           - Run the application (cargo run)"
	@echo "  test          - Run unit tests (cargo test)"
	@echo "  cov           - Run code coverage (rustup run nightly cargo llvm-cov)"
	@echo "  crap          - Enforce CRAP score gatekeeper (cargo crap)"
	@echo "  mutants       - Run mutation testing (cargo mutants)"
	@echo "  fmt           - Check code formatting (cargo fmt)"
	@echo "  lint          - Run clippy lints (cargo clippy)"
	@echo "  pre-commit    - Run all pre-commit checks (fmt, lint, crap)"
	@echo "  install-hooks - Install git pre-commit hook"
	@echo "  setup-tools   - Install required CLI tools (cargo-llvm-cov, cargo-crap, cargo-mutants)"
	@echo "  clean         - Clean build artifacts (cargo clean)"

