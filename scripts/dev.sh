#!/usr/bin/env bash
# Development: three watchers, one Ctrl-C.
#
#   1. Vite dev server   http://127.0.0.1:5173   React Fast Refresh (HMR); proxies /api -> :8000
#   2. uvicorn --reload  http://127.0.0.1:8000   restarts on app/*.py changes or a rebuilt Rust .so
#   3. Rust watcher      rust/src/*.rs changes -> `uv sync` rebuilds + reinstalls the extension,
#                        which in turn triggers (2) to restart with the new native module.
#
# Open http://127.0.0.1:5173 (NOT :8000) while developing.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_PORT="${API_PORT:-8000}"

cd "$ROOT/backend"
uv sync --quiet
CORE_DIR="$(uv run --no-sync python -c 'import os, viewfinder_core; print(os.path.dirname(viewfinder_core.__file__))')"
[ -d "$ROOT/frontend/node_modules" ] || (cd "$ROOT/frontend" && npm install)

# On exit, signal our whole process group so uv/node/uvicorn grandchildren die too.
trap 'trap - INT TERM EXIT; kill 0 2>/dev/null' INT TERM EXIT

# (3) Rust watcher. `watchfiles` ships with uvicorn[standard]. `uv sync` is a no-op
# unless the cache-keys in backend/pyproject.toml (Cargo.toml, src/**/*.rs) changed.
(cd "$ROOT/backend" && uv run --no-sync watchfiles --target-type command \
    "uv sync --quiet" "$ROOT/rust/src" 2>&1 | sed -u 's/^/[rust] /') &

# (2) API with auto-reload. Watches app/ for .py and the installed extension dir for .so.
(cd "$ROOT/backend" && uv run --no-sync uvicorn app.main:app --host 127.0.0.1 --port "$API_PORT" \
    --reload --reload-dir app --reload-dir "$CORE_DIR" --reload-include '*.so' 2>&1 | sed -u 's/^/[api]  /') &

# (1) Vite with HMR.
(cd "$ROOT/frontend" && VITE_API_TARGET="http://127.0.0.1:$API_PORT" npm run dev 2>&1 | sed -u 's/^/[web]  /') &

wait -n  # if any one of the three exits, tear everything down
