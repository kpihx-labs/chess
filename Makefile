.PHONY: help check install push tag push-tags release

VERSION := $(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
TAG := v$(VERSION)

help:
	@printf '%s\n' \
	  'make check       Format, lint, test, release-build, and smoke-test' \
	  'make install     Install the native chess binary through Cargo' \
	  'make push        Push main to GitHub and GitLab' \
	  'make release     Check, push, tag, and push the current tag'

check:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --all-targets --all-features
	cargo build --release --locked
	cargo run --locked -- --help
	cargo run --locked -- --version

install: check
	cargo install --path . --force --locked

push:
	git push github main
	git push gitlab main

tag:
	git rev-parse --verify "refs/tags/$(TAG)" >/dev/null 2>&1 || git tag -a "$(TAG)" -m "Release $(TAG)"

push-tags:
	git push github "refs/tags/$(TAG)"
	git push gitlab "refs/tags/$(TAG)"

release: check push tag push-tags
