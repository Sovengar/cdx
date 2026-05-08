#!/usr/bin/env bash
# cdx install script — Linux / macOS
# Usage: curl -fsSL https://raw.githubusercontent.com/Sovengar/cdx-rs/main/scripts/install.sh | bash
# Or:   ./scripts/install.sh
set -euo pipefail

REPO="Sovengar/cdx-rs"
BIN_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/cdx"
BIN_NAME="cdx"

mkdir -p "${BIN_DIR}" "${CONFIG_DIR}"

# Check for Rust
if ! command -v rustc &>/dev/null; then
    echo "[cdx] Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "${HOME}/.cargo/env"
fi

# Clone or update
PROJECT_DIR=$(mktemp -d)
trap 'rm -rf "${PROJECT_DIR}"' EXIT

git clone "https://github.com/${REPO}.git" "${PROJECT_DIR}"
cd "${PROJECT_DIR}"

echo "[cdx] Building release..."
cargo build --release

# Install binary
cp "target/release/cdx-rs" "${BIN_DIR}/${BIN_NAME}"
chmod +x "${BIN_DIR}/${BIN_NAME}"

# Create default config if not exists
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    "${BIN_DIR}/${BIN_NAME}" --version 2>/dev/null || true
    if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
        cat > "${CONFIG_DIR}/config.toml" << 'EOF'
# cdx config — see https://github.com/Sovengar/cdx-rs
show_dotfiles = false
show_winhidden = false
EOF
    fi
fi

# Add to PATH hint
case ":${PATH}:" in
    *:"${BIN_DIR}":*) ;;
    *) echo "[cdx] Add ${BIN_DIR} to your PATH (e.g. in ~/.bashrc)" ;;
esac

echo "[cdx] Installed! Run 'cdx' to start."
