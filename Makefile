.PHONY: help check-all check-frontend check-rust check-lua test-lua build-kjn desktop-dev desktop-build fmt gen

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

check-lua: ## Lua format check + type check
	stylua --check lua/ plugin/ tests/
	lua-language-server --check .

test-lua: ## Run Neovim plugin tests
	nvim --headless --noplugin -u tests/minimal_init.lua -c "lua MiniTest.run()"

test-lua-e2e: build-kjn ## Run Neovim plugin e2e tests (requires jj)
	nvim --headless --noplugin -u tests/minimal_init.lua \
		-c "lua MiniTest.run_file('tests/e2e/test_kjn_calls.lua')"
	nvim --headless --noplugin -u tests/minimal_init.lua \
		-c "lua MiniTest.run_file('tests/e2e/test_e2e_review.lua')"
	nvim --headless --noplugin -u tests/minimal_init.lua \
		-c "lua MiniTest.run_file('tests/e2e/test_e2e_log.lua')"
	nvim --headless --noplugin -u tests/minimal_init.lua \
		-c "lua MiniTest.run_file('tests/e2e/test_e2e_file_tree.lua')"

build-kjn: ## Build Neovim plugin binary
	cargo build --release --bin kjn

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
