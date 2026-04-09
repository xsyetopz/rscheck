.PHONY: check lint fmt test

check:
	cargo check --workspace && cargo check --workspace --tests

lint:
	cargo clippy --workspace && cargo check --workspace --tests

fmt:
	cargo fmt --all

test:
	cargo test --workspace
