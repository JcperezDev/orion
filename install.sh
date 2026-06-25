#!/bin/bash
set -e

# Detect install dir (default ~/.local/bin, fall back to ~/.cargo/bin)
INSTALL_DIR="${ORION_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"

echo "Installing ORION..."
echo "  build target: release"
echo "  install dir:  $INSTALL_DIR"

# Build the CLI and server in release mode
cargo build --release -p orion -p orion-server 2>/dev/null \
    || cargo build --release -p orion -p orion-server

# Copy binaries
if [ -f target/release/orion ]; then
    cp target/release/orion "$INSTALL_DIR/orion"
    chmod +x "$INSTALL_DIR/orion"
    echo "✓ installed orion (CLI)"
else
    echo "✗ target/release/orion not found — build failed?"
    exit 1
fi

if [ -f target/release/orion-server ]; then
    cp target/release/orion-server "$INSTALL_DIR/orion-server"
    chmod +x "$INSTALL_DIR/orion-server"
    echo "✓ installed orion-server"
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
echo "Usage:"
echo "  orion              # launch TUI (default)"
echo "  orion run \"...\"    # headless prompt"
echo "  orion serve        # start HTTP server on :7337"
echo "  orion providers    # list configured providers"
echo "  orion connect openai   # save API key for openai"
echo "  orion sessions     # list recent chat sessions"
echo "  orion init         # write AGENTS.md to current dir"