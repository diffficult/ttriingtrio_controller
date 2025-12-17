#!/bin/bash
# Installation script for Riing Trio RGB Controller

set -e

echo "=== Riing Trio RGB Controller - Installation ==="
echo

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✓ Rust/Cargo found"
echo

# Build
echo "Building release binary..."
cargo build --release
echo "✓ Build complete"
echo

# Check if device is connected
echo "Checking for Thermaltake device..."
if lsusb -d 264a: &> /dev/null; then
    echo "✓ Thermaltake device found:"
    lsusb -d 264a:
else
    echo "⚠ No Thermaltake device found (VID: 264a)"
    echo "  This is OK if the device is not currently connected"
fi
echo

# Offer to install udev rule
echo "Would you like to install udev rules for non-root access? [y/N]"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    if [ "$EUID" -ne 0 ]; then
        echo "Installing udev rule (requires sudo)..."
        sudo cp 99-thermaltake.rules /etc/udev/rules.d/
        sudo udevadm control --reload-rules
        sudo udevadm trigger
        echo "✓ udev rule installed"
        echo "  Please unplug and replug your device for the rule to take effect"
    else
        echo "Installing udev rule..."
        cp 99-thermaltake.rules /etc/udev/rules.d/
        udevadm control --reload-rules
        udevadm trigger
        echo "✓ udev rule installed"
        echo "  Please unplug and replug your device for the rule to take effect"
    fi
else
    echo "Skipping udev rule installation"
    echo "  You can install it later with:"
    echo "    sudo cp 99-thermaltake.rules /etc/udev/rules.d/"
    echo "    sudo udevadm control --reload-rules"
    echo "    sudo udevadm trigger"
fi
echo

# Show binary location
echo "=== Installation Complete ==="
echo
echo "Binary location: $(pwd)/target/release/riing-trio-controller"
echo
echo "Usage examples:"
echo "  ./target/release/riing-trio-controller --port 1 off"
echo "  ./target/release/riing-trio-controller --port 1 white"
echo
echo "For more information, see README.md"
