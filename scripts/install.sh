#!/usr/bin/env bash
# cdx install script — Linux / macOS
# Usage: curl -fsSL https://raw.githubusercontent.com/Sovengar/cdx/main/scripts/install.sh | bash
# Or:   ./scripts/install.sh
set -euo pipefail

REPO="Sovengar/cdx"
BIN_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/cdx"
BIN_NAME="cdx"

echo "[cdx] Installing..."

mkdir -p "${BIN_DIR}" "${CONFIG_DIR}"

# Check for Rust
if ! command -v rustc &>/dev/null; then
    echo "[cdx] Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1091
    source "${HOME}/.cargo/env"
fi

# Clone to temp dir
PROJECT_DIR=$(mktemp -d)
trap 'rm -rf "${PROJECT_DIR}"' EXIT

echo "[cdx] Cloning repository..."
git clone --depth 1 "https://github.com/${REPO}.git" "${PROJECT_DIR}"
cd "${PROJECT_DIR}"

echo "[cdx] Building release binary..."
cargo build --release

# Install binary
cp "target/release/${BIN_NAME}" "${BIN_DIR}/${BIN_NAME}"
chmod +x "${BIN_DIR}/${BIN_NAME}"

# Create default config if not exists
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    "${BIN_DIR}/${BIN_NAME}" --version 2>/dev/null || true
    if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
        cat > "${CONFIG_DIR}/config.toml" << 'EOF'
# cdx config — see https://github.com/Sovengar/cdx
show_dotfiles = false
show_winhidden = false
EOF
    fi
fi

# Add to PATH if not present
add_to_path() {
    local shell_rc="$1"
    local export_line="export PATH=\"${BIN_DIR}:\$PATH\""

    if [ -f "${shell_rc}" ] && ! grep -q "${BIN_DIR}" "${shell_rc}"; then
        echo "" >> "${shell_rc}"
        echo "# cdx" >> "${shell_rc}"
        echo "${export_line}" >> "${shell_rc}"
        echo "[cdx] Added ${BIN_DIR} to ${shell_rc}"
    fi
}

case ":${PATH}:" in
    *:"${BIN_DIR}":*) ;;
    *)
        # Detect shell and add to appropriate rc file
        CURRENT_SHELL=$(basename "${SHELL:-/bin/bash}")
        case "${CURRENT_SHELL}" in
            bash) add_to_path "${HOME}/.bashrc" ;;
            zsh)  add_to_path "${HOME}/.zshrc" ;;
            fish)
                FISH_DIR="${HOME}/.config/fish/conf.d"
                mkdir -p "${FISH_DIR}"
                if [ ! -f "${FISH_DIR}/cdx.fish" ]; then
                    echo "set -gx PATH ${BIN_DIR} \$PATH" > "${FISH_DIR}/cdx.fish"
                    echo "[cdx] Added ${BIN_DIR} to fish PATH"
                fi
                ;;
            *)    echo "[cdx] Add ${BIN_DIR} to your PATH" ;;
        esac
        ;;
esac

echo ""
echo "[cdx] ✓ Installed! Run 'cdx' to start."
echo "[cdx] Reload your shell or run: source ~/.${CURRENT_SHELL:-bash}rc"
