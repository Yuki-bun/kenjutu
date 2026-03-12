.PHONY: help check-all check-frontend check-rust check-lua test-lua build-kjn install-kjn desktop-dev desktop-build fmt gen

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

check-all: check-frontend check-rust check-lua ## Run all checks

check-frontend: ## Frontend checks (tsc + eslint + prettier)
	pnpm exec prettier --check .
	pnpm run check
	pnpm run lint

check-rust: ## Rust checks (fmt + clippy + tests)
	cargo fmt --check
	cargo clippy --workspace -- -D warnings 
	cargo test --workspace

check-lua: check-lua-fmt lua-type-check test-lua ## Lua checks (format + type check + tests)

check-lua-fmt:
	stylua --check lua/ plugin/ tests/

lua-type-check:
	lua-language-server --check .

test-lua: ## Run Neovim plugin tests
	nvim -l tests/run.lua

build-kjn: ## Build Neovim plugin binary from source
	cargo build --release --bin kjn

install-kjn: ## Download prebuilt kjn binary
	nvim -l lua/kenjutu/install.lua

desktop-dev: ## Start Tauri dev mode
	pnpm tauri dev

desktop-build: ## Tauri production build
	pnpm i
	pnpm tauri build

fmt: ## Format all (JS + Rust + Lua)
	pnpm fmt
	cargo fmt
	stylua lua/ plugin/ tests/

gen: ## Generate TS bindings
	cargo run --bin bindings
