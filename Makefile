# cdx build/test gates.
#
# CI invokes these exact targets, so the local gate and the CI gate are the same
# by construction. Keep them in sync with .github/workflows/ci.yml.

.PHONY: build lint test check

build:
	cargo build --locked --all-targets

lint:
	cargo clippy --locked --all-targets -- -D warnings

test:
	cargo test --locked

check: build lint test
