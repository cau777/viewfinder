# One-stop commands. Requires: rustup (stable), uv, node >= 20, python3-dev (for `cargo test`).
SHELL := /bin/bash
.DEFAULT_GOAL := help

PORT    ?= 8000
HOST    ?= 0.0.0.0
WORKERS ?= 2

help: ## List targets
	@grep -hE '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  \033[36m%-10s\033[0m %s\n", $$1, $$2}'

setup: ## Install everything (Python venv + Rust extension build + node_modules)
	cd backend && uv sync
	cd frontend && npm install

dev: ## Hot-reload dev stack: Vite :5173 (HMR) + uvicorn :8000 (--reload) + Rust auto-rebuild
	./scripts/dev.sh

dev-remote: ## Vite only, with /api proxied to the API running on the JupyterHub server (JUPYTER_* in .env)
	cd frontend && VIEWFINDER_REMOTE_API=1 npm run dev

notebook: ## JupyterLab on notebooks/, using the backend venv (imports app.* and viewfinder_core)
	cd backend && uv sync --quiet && uv run --no-sync jupyter lab --notebook-dir ../notebooks

notebook-check: ## Execute every notebook top to bottom (fails on any error); does not modify files
	cd backend && for nb in ../notebooks/*.ipynb; do \
		uv run --no-sync jupyter nbconvert --to notebook --execute --stdout "$$nb" >/dev/null || exit 1; done

stubs: ## Regenerate rust/python/viewfinder_core/_native.pyi from the compiled Rust code
	rm -rf rust/target/stubs
	cd rust && uvx maturin generate-stubs --quiet --out target/stubs
	cp rust/target/stubs/viewfinder_core/_native.pyi rust/python/viewfinder_core/_native.pyi

stubs-check: ## Fail if the committed .pyi is out of date with lib.rs
	rm -rf rust/target/stubs
	cd rust && uvx maturin generate-stubs --quiet --out target/stubs
	@diff -u rust/python/viewfinder_core/_native.pyi rust/target/stubs/viewfinder_core/_native.pyi \
		|| (echo "Stubs are stale: run 'make stubs' and commit the result." && exit 1)

test: stubs-check ## Rust unit tests + stub freshness + Python/API tests + TS typecheck/lint + notebooks
	cd rust && cargo test --quiet
	cd backend && uv sync --quiet && uv run pytest -q
	cd frontend && npx tsc -b && npm run lint
	$(MAKE) --no-print-directory notebook-check

build: ## Production build: optimized Rust extension + static React bundle
	cd backend && uv sync --no-default-groups --reinstall-package viewfinder-core
	cd frontend && npm ci && npm run build

serve: ## Run the production single process (API + SPA) on $(HOST):$(PORT)
	cd backend && uv run --no-sync uvicorn app.main:app --host $(HOST) --port $(PORT) --workers $(WORKERS) --proxy-headers

wheel: ## Build a distributable viewfinder-core wheel into dist-wheels/
	cd rust && uvx maturin build --release --out ../dist-wheels

release: wheel ## Self-contained tarball for a host with only Python >= 3.10 (no Rust/Node)
	cd frontend && npm ci && npm run build
	rm -rf dist-release && mkdir -p dist-release/viewfinder/backend dist-release/viewfinder/frontend
	cd backend && uv export --frozen --no-default-groups --no-hashes --no-emit-project \
		--no-emit-package viewfinder-core > ../dist-release/viewfinder/requirements.txt
	cp -r backend/app dist-release/viewfinder/backend/
	cp -r frontend/dist dist-release/viewfinder/frontend/
	cp -r dist-wheels dist-release/viewfinder/wheels
	tar czf dist-release/viewfinder.tgz -C dist-release viewfinder
	@echo "-> dist-release/viewfinder.tgz"

clean: ## Remove build artifacts
	rm -rf rust/target frontend/dist dist-wheels dist-release backend/.venv

.PHONY: help setup dev dev-remote notebook notebook-check stubs stubs-check test build serve wheel release clean
