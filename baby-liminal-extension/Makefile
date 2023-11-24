.PHONY: test test-frontend test-backend lint clippy clean help

AT_TEST_CONTRACT = --manifest-path test_contract/Cargo.toml
TEST_TOOLCHAIN = nightly-2023-05-22-unknown-linux-gnu

test: test-frontend test-backend ## Run all tests

test-frontend: ## Run frontend tests
	cargo test --release $(AT_TEST_CONTRACT) || exit 1

test-backend: ## Run backend tests
	cargo +$(TEST_TOOLCHAIN) test --release --features runtime-std || exit 1

lint: ## Check linting rules
	cargo +nightly fmt --all --check || exit 1
	cargo +nightly fmt --all --check $(AT_TEST_CONTRACT) || exit 1

clippy: ## Run clippy
	cargo clippy --no-deps --features ink-std -- -D warnings || exit 1
	cargo clippy --no-deps --features runtime-std -- -D warnings || exit 1

clean: ## Clean all build files
	cargo clean
	cargo clean $(AT_TEST_CONTRACT)

help: ## Displays this help
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[1;36m<target>\033[0m\n\nTargets:\n"} /^[a-zA-Z0-9_-]+:.*?##/ { printf "  \033[1;36m%-25s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
