#!/bin/bash
# ORION installer.
#
#   From source (inside a clone):   ./install.sh
#   Download a release (one-liner):
#     curl -fsSL https://raw.githubusercontent.com/JcperezDev/orion/master/install.sh | bash
#
# Override the install dir with ORION_INSTALL_DIR=/path.
# Force a mode with ORION_INSTALL_MODE=build|download.
set -euo pipefail

REPO="JcperezDev/orion"
INSTALL_DIR="${ORION_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"

# Build from source only when run inside the repo with cargo available;
# otherwise download a prebuilt release.
MODE="${ORION_INSTALL_MODE:-}"
if [ -z "$MODE" ]; then
  if [ -f "Cargo.toml" ] && grep -q 'orion-core' Cargo.toml 2>/dev/null && command -v cargo >/dev/null 2>&1; then
    MODE="build"
  else
    MODE="download"
  fi
fi

echo "Installing ORION (mode: $MODE, dir: $INSTALL_DIR)"

if [ "$MODE" = "build" ]; then
  cargo build --release -p orion -p orion-server
  install -m 0755 target/release/orion "$INSTALL_DIR/orion"
  echo "✓ installed orion (CLI)"
  if [ -f target/release/orion-server ]; then
    install -m 0755 target/release/orion-server "$INSTALL_DIR/orion-server"
    echo "✓ installed orion-server"
  fi
else
  # --- Download a prebuilt release for this OS/arch ---
  os="$(uname -s)"; arch="$(uname -m)"
  case "$os" in
    Linux)  os_part="unknown-linux-gnu" ;;
    Darwin) os_part="apple-darwin" ;;
    *) echo "✗ unsupported OS '$os' for the binary installer; build from source instead." >&2; exit 1 ;;
  esac
  case "$arch" in
    x86_64|amd64) arch_part="x86_64" ;;
    arm64|aarch64) arch_part="aarch64" ;;
    *) echo "✗ unsupported architecture '$arch'." >&2; exit 1 ;;
  esac
  target="${arch_part}-${os_part}"
  url="https://github.com/${REPO}/releases/latest/download/orion-${target}.tar.gz"

  echo "  downloading ${url}"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  if ! curl -fSL "$url" -o "$tmp/orion.tar.gz"; then
    echo "✗ download failed — is there a published release for ${target}?" >&2
    echo "  See https://github.com/${REPO}/releases" >&2
    exit 1
  fi
  tar -xzf "$tmp/orion.tar.gz" -C "$tmp"
  src="$tmp/orion-${target}"
  install -m 0755 "$src/orion" "$INSTALL_DIR/orion"
  echo "✓ installed orion (CLI)"
  if [ -f "$src/orion-server" ]; then
    install -m 0755 "$src/orion-server" "$INSTALL_DIR/orion-server"
    echo "✓ installed orion-server"
  fi
fi

# PATH check
case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    echo ""
    echo "✓ $INSTALL_DIR is already on PATH"
    ;;
  *)
    echo ""
    echo "Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac

echo ""
echo "Run: orion"
