#!/bin/bash
set -e

INSTALL_DIR="${HOME}/.cargo/bin"
mkdir -p "$INSTALL_DIR"

echo "Installing ORION..."
cargo build --release 2>/dev/null || cargo build --release

cp target/release/orion-agent "$INSTALL_DIR/orion"
chmod +x "$INSTALL_DIR/orion"

echo "ORION installed to $INSTALL_DIR/orion"
echo "Add $INSTALL_DIR to your PATH if needed"
