# Quick Start Guide

## 1. Prerequisites
```bash
# Verify Rust is installed
cargo --version

# If not installed:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## 2. Build
```bash
cd riing-trio-controller
cargo build --release
```

## 3. Run (First Time)
```bash
# With sudo (if no udev rule)
sudo ./target/release/riing-trio-controller --port 1 off
```

## 4. Install udev Rule (Optional but Recommended)
```bash
sudo cp 99-thermaltake.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger

# Unplug and replug device
```

## 5. Run (After udev Rule)
```bash
# No sudo needed!
./target/release/riing-trio-controller --port 1 white
./target/release/riing-trio-controller --port 1 off
```

## Quick Commands
```bash
# Turn off all LEDs on port 1
./target/release/riing-trio-controller --port 1 off

# Set white on port 2
./target/release/riing-trio-controller --port 2 white

# Control all ports
for p in {1..5}; do
    ./target/release/riing-trio-controller --port $p white
done
```

## Troubleshooting
- **Device not found**: Run `lsusb | grep 264a` to verify device is connected
- **Permission denied**: Use `sudo` or install udev rule
- **Wrong PID**: Use `lsusb -d 264a:` to find your PID, then use `--pid 0xXXXX`

See README.md for detailed documentation.
