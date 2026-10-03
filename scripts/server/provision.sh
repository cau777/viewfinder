#!/usr/bin/env bash
# One-time, root-only setup of the shared team server (TLJH container) so any user can
# `git clone` this repo and run `make setup`. Idempotent: safe to re-run after updates.
#
#   sudo scripts/server/provision.sh
#
# Installs, for every user:
#   - Node 22 (system apt nodejs is 18; Vite needs >= 20)
#   - uv in /usr/local/bin
#   - Rust (rustup, stable) in /opt/rust; each user keeps their own registry cache in ~/.cargo
#   - the shared "Python (viewfinder backend)" Jupyter kernel, which runs the *calling user's*
#     ~/viewfinder-scaffold/backend/.venv (see viewfinder-kernel)
set -euo pipefail

[[ $EUID -eq 0 ]] || { echo "Run as root: sudo $0" >&2; exit 1; }
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TLJH_PREFIX=/opt/tljh/user
RUST_ROOT=/opt/rust
export DEBIAN_FRONTEND=noninteractive

echo "==> Node >= 20"
if ! node -v 2>/dev/null | grep -qE '^v(2[0-9]|[3-9][0-9])\.'; then
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - >/dev/null
    apt-get install -y -qq nodejs >/dev/null
fi
node -v

echo "==> uv"
if [[ ! -x /usr/local/bin/uv ]]; then
    curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR=/usr/local/bin UV_NO_MODIFY_PATH=1 sh >/dev/null
fi
/usr/local/bin/uv --version

echo "==> Rust (shared toolchain in $RUST_ROOT)"
export RUSTUP_HOME=$RUST_ROOT/rustup CARGO_HOME=$RUST_ROOT/cargo
if [[ ! -x $CARGO_HOME/bin/rustup ]]; then
    curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable >/dev/null
else
    "$CARGO_HOME/bin/rustup" update stable >/dev/null
fi
chmod -R a+rX "$RUST_ROOT"
"$CARGO_HOME/bin/rustc" --version

# RUSTUP_HOME is shared and read-only for users; CARGO_HOME stays unset so each user's
# downloads go to ~/.cargo. Appended to PATH so a personal rustup (~/.cargo/bin) still wins.
cat > /etc/profile.d/01-shared-rust.sh <<EOF
# Shared Rust toolchain (installed by viewfinder-scaffold/scripts/server/provision.sh)
export RUSTUP_HOME=$RUST_ROOT/rustup
case ":\$PATH:" in *:$RUST_ROOT/cargo/bin:*) ;; *) export PATH="\$PATH:$RUST_ROOT/cargo/bin" ;; esac
EOF
# Non-login shells (Jupyter and PyCharm terminals) only read bash.bashrc.
grep -q 01-shared-rust /etc/bash.bashrc || printf '\n. /etc/profile.d/01-shared-rust.sh\n' >> /etc/bash.bashrc

echo "==> Shared Jupyter kernel"
install -m 755 "$HERE/viewfinder-kernel" /usr/local/bin/viewfinder-kernel
install -D -m 644 "$HERE/missing_venv.py" /usr/local/share/viewfinder-kernel/missing_venv.py
KDIR=$TLJH_PREFIX/share/jupyter/kernels/viewfinder-backend
mkdir -p "$KDIR"
cp "$TLJH_PREFIX"/share/jupyter/kernels/python3/logo-* "$KDIR"/ 2>/dev/null || true
cat > "$KDIR/kernel.json" <<'EOF'
{
  "argv": ["/usr/local/bin/viewfinder-kernel", "-f", "{connection_file}"],
  "display_name": "Python (viewfinder backend)",
  "language": "python",
  "metadata": {"debugger": true}
}
EOF
"$TLJH_PREFIX/bin/jupyter" kernelspec list

echo "Done. Users: open a new terminal, then: git clone ... ~/viewfinder-scaffold && make setup"
