.PHONY: check lint fmt test site-dev site-build site-check

check:
	cargo check --workspace && cargo check --workspace --tests

lint:
	cargo clippy --workspace --all-targets -- -D warnings && cargo check --workspace --tests

fmt:
	cargo fmt --all

test:
	cargo test --workspace

site-dev:
	bun run site:dev

site-build:
	bun run site:build

site-check:
	bun run biome:check && bun run typecheck && bun run typecheck:build && bun run site:build
