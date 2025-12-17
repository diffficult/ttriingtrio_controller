# Riing Trio RGB Controller

A comprehensive Linux CLI tool to control Thermaltake Riing Trio RGB fans via HID USB. Supports **full LED effects**, **custom colors**, **brightness control**, **fan speed control**, and **persistent settings via daemon mode**.

## Features

### 🎨 LED Effects (30 FPS Animations)
- ✅ **Spectrum** - Rainbow color wheel cycling
- ✅ **Wave** - Traveling wave patterns
- ✅ **Pulse** - Breathing/fading effects
- ✅ **Blink** - On/off blinking
- ✅ **Flow** - Multi-color chasing
- ✅ **Ripple** - Expanding wave patterns
- ✅ **Static** - Solid colors

### 🎭 Colors & Brightness
- ✅ **13 predefined colors**: off, white, red, green, blue, cyan, magenta, yellow, orange, purple, pink, lime, sky
- ✅ **Brightness control**: 0-100% adjustable intensity
- ✅ **Effect speeds**: Extreme, Fast, Normal, Slow

### ⚙️ System Features
- ✅ **Fan Speed Control**: 0-100% (minimum ~500 RPM)
- ✅ **Status Monitoring**: Read current RPM and speed
- ✅ **Daemon Mode**: Continuously apply settings to prevent controller reset
- ✅ **Systemd Integration**: Run as a system service
- ✅ **Direct HID USB communication** (no kernel drivers needed)
- ✅ **Per-port control** (ports 1-5)
- ✅ **Configurable LED count**

## Hardware Support

- **Vendor ID**: `0x264a`
- **Product ID**: `0x2135` (default) to `0x2144` (configurable)
- **Ports**: 1-5
- **Default LED Count**: 30 per port (Riing Trio fans)

## Requirements

### System Requirements
- **OS**: Linux with hidraw support
- **Rust**: 1.70 or newer
- **Permissions**: Root access OR udev rule (recommended)

### Dependencies
All dependencies are managed by Cargo:
- `hidapi` - HID device access
- `clap` - CLI argument parsing
- `anyhow` - Error handling
- `serde` - Configuration serialization
- `toml` - Configuration file parsing
- `chrono` - Timestamp formatting

## Installation

### 1. Clone or Extract
If you received this as a file, you already have it. Otherwise:
```bash
# Ensure you're in the project directory
cd riing-trio-controller
```

### 2. Build
```bash
cargo build --release
```

The binary will be at: `target/release/riing-trio-controller`

### 3. Set Up Permissions (Recommended)

#### Option A: udev Rule (Recommended)
Create a udev rule to allow non-root access:

```bash
# Create the rule file
sudo tee /etc/udev/rules.d/99-thermaltake.rules > /dev/null << 'EOF'
# Thermaltake Riing Trio Controller
# VID=0x264a, PID=0x2135-0x2144
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2135", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2136", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2137", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2138", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2139", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="213a", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="213b", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="213c", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="213d", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="213e", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="213f", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2140", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2141", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2142", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2143", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="264a", ATTRS{idProduct}=="2144", MODE="0666"
EOF

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

After this, you can run the tool without sudo.

#### Option B: Run with sudo
```bash
sudo ./target/release/riing-trio-controller --port 1 off
```

## Usage

### Quick Start - Effects

```bash
# Create config with spectrum effect
cat > my-config.toml << 'EOF'
[daemon]
interval_seconds = 5

[ports.1]
speed = 50
effect = "spectrum"
effect_speed = "normal"
EOF

# Run daemon (smooth rainbow at 30 FPS!)
sudo ./target/release/riing-trio-controller daemon --config my-config.toml
```

### LED Control

```bash
# Turn off LEDs on port 1
./target/release/riing-trio-controller off --port 1

# Set LEDs to white on port 2
./target/release/riing-trio-controller white --port 2
```

### Fan Speed Control

```bash
# Set fan speed to 50% on port 1
./target/release/riing-trio-controller speed --port 1 --speed 50

# Set fan speed to 100% on port 3
./target/release/riing-trio-controller speed --port 3 --speed 100

# Set minimum speed (fans still spin)
./target/release/riing-trio-controller speed --port 1 --speed 0
```

### Status Monitoring

```bash
# Check status of port 1 (shows RPM and current speed)
./target/release/riing-trio-controller status --port 1

# Check all ports
./target/release/riing-trio-controller status
```

Example output:
```
Port 1:
  Speed: 50%
  RPM: 1245

Port 2:
  Speed: 75%
  RPM: 1867
```

### Daemon Mode with Effects

**Example 1: Spectrum (Rainbow)**
```toml
[daemon]
interval_seconds = 5

[ports.1]
speed = 50
effect = "spectrum"
effect_speed = "normal"
```

**Example 2: Wave Effect**
```toml
[ports.1]
speed = 60
effect = "wave"
color = "cyan"
effect_speed = "fast"
```

**Example 3: Breathing/Pulse**
```toml
[ports.1]
speed = 40
effect = "pulse"
color = "purple"
effect_speed = "slow"
brightness = 0.8
```

**Example 4: Flow (Multi-color)**
```toml
[ports.1]
speed = 75
effect = "flow"
flow_colors = "red, orange, yellow, green, blue, purple"
effect_speed = "normal"
```

**Example 5: Static with Brightness**
```toml
[ports.1]
speed = 50
effect = "static"
color = "white"
brightness = 0.5  # 50% brightness
```

See **[EFFECTS_GUIDE.md](EFFECTS_GUIDE.md)** for complete effects documentation!

### Advanced Options

```bash
# Specify custom PID (if your device uses a different product ID)
./target/release/riing-trio-controller --pid 0x2136 off --port 1

# Custom LED count (if not using standard Riing Trio fans)
./target/release/riing-trio-controller white --port 1 --led-count 40
```

## Making Settings Persistent

**Problem:** The Thermaltake controller resets to default (rainbow LEDs, default fan speed) after ~7 seconds without commands.

**Solution:** Use **daemon mode** to continuously reapply your settings every 5 seconds.

### Quick Setup

1. **Create config file:**
   ```bash
   cp riing-config.toml my-config.toml
   nano my-config.toml
   ```

2. **Edit configuration:**
   ```toml
   [daemon]
   interval_seconds = 5

   [ports.1]
   speed = 50
   color = "white"

   [ports.2]
   speed = 75
   color = "off"
   ```

3. **Run daemon:**
   ```bash
   sudo ./target/release/riing-trio-controller daemon --config my-config.toml
   ```

4. **Make it permanent (systemd):**
   ```bash
   sudo cp target/release/riing-trio-controller /usr/local/bin/
   sudo cp riing-config.toml /etc/riing-config.toml
   sudo cp riing-trio-controller.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable --now riing-trio-controller.service
   ```

**See [DAEMON_MODE.md](DAEMON_MODE.md) for complete instructions.**

### Full Command Reference

```
riing-trio-controller [OPTIONS] <COMMAND>

Commands:
  off     Turn off all LEDs on the specified port
  white   Set all LEDs to white on the specified port
  speed   Set fan speed (0-100%)
  status  Show current status (RPM, speed) for a port
  daemon  Run as daemon, continuously applying settings from config file

Global Options:
      --vid <VID>  USB Vendor ID [default: 0x264a]
      --pid <PID>  USB Product ID [default: 0x2135]
  -h, --help       Print help

Command-Specific Options:
  off/white:
    -p, --port <PORT>           Port number (1-5)
        --led-count <LED_COUNT> Number of LEDs per port [default: 30]

  speed:
    -p, --port <PORT>   Port number (1-5)
    -s, --speed <SPEED> Speed percentage (0-100)

  status:
    -p, --port <PORT>   Port number (1-5), or omit to show all ports

  daemon:
    -c, --config <CONFIG>     Path to configuration file [default: riing-config.toml]
    -i, --interval <INTERVAL> Interval in seconds [default: 5]
```

## Protocol Details

This implementation is grounded in the [TTController C# reference](https://github.com/MoshiMoshi0/TTController).

### Key Protocol Facts
- **Report Size**: 65 bytes (1 byte report ID + 64 bytes payload)
- **Report ID**: 0x00 (always)
- **Timeout**: 1000ms for device responses
- **Status Byte**: Response byte 3 indicates success (0xFC) or failure (0xFE)

### Commands Implemented

#### 1. Init
```
Payload: [0xFE, 0x33]
Response: Check byte[2] == 0xFC (Linux) or byte[3] == 0xFC (Windows)
```

**Note:** On Linux, the hidraw driver strips the report ID on read operations, so the status byte is at index 2 instead of index 3 (as documented in the C# Windows implementation).

#### 2. Set RGB (PerLed Mode)
```
Payload: [0x32, 0x52, PORT, 0x24, 0x03, CHUNK_ID, 0x00, COLORS...]
Mode: 0x24 = PerLed
Colors: GRB order (Green, Red, Blue) - NOT RGB!
Chunking: Max 19 colors per chunk, 2 chunks used for 30 LEDs
Response: Check byte[3] == 0xFC after each chunk
```

**IMPORTANT**: Colors are sent in **GRB order**, not RGB. This is a quirk of the Thermaltake protocol.

### LED Count & Chunking
- **Default**: 30 LEDs per port (Riing Trio)
- **Zones**: [12, 12, 6] LEDs per ring (but treated as one zone for solid colors)
- **Chunking**: 19 colors max per chunk = 2 chunks for 30 LEDs
- Maximum theoretical: 76 LEDs (4 chunks × 19)

## Troubleshooting

### Device Not Found
```
Error: Failed to open HID device 264a:2135
```

**Solutions:**
1. Check if device is connected: `lsusb | grep 264a`
2. Verify the PID: `lsusb -v -d 264a:` (look for your specific PID)
3. Try with sudo: `sudo ./target/release/riing-trio-controller --port 1 off`
4. Install udev rule (see above)

### Permission Denied
```
Error: Failed to open HID device: Permission denied
```

**Solutions:**
1. Run with sudo: `sudo ./target/release/riing-trio-controller --port 1 off`
2. Install udev rule (recommended, see Installation section)
3. Check device permissions: `ls -l /dev/hidraw*`

### Init Failure
```
Error: Init failed: Device returned error (0xFE)
```

**Solutions:**
1. Disconnect and reconnect the USB device
2. Ensure no other software is accessing the controller
3. Try a different USB port
4. Check if device supports your PID: try nearby PIDs with `--pid 0x2136`

### Timeout
```
Error: Timeout: No response from device after 1000ms
```

**Solutions:**
1. Device may be busy - wait a few seconds and retry
2. Check USB cable connection
3. Try different USB port (prefer USB 2.0 ports)
4. Verify device is not in sleep/power-save mode

### Wrong PID
```
Error: Failed to open HID device 264a:2135
```

If your device uses a different PID in the range 0x2135-0x2144:
```bash
# Find your device PID
lsusb | grep 264a

# Use the correct PID
./target/release/riing-trio-controller --pid 0x2136 --port 1 off
```

### Checking Device
```bash
# List all USB devices
lsusb

# Find Thermaltake devices
lsusb | grep -i thermaltake
lsusb -d 264a:

# Check hidraw devices
ls -l /dev/hidraw*

# Check which hidraw belongs to Thermaltake
for dev in /dev/hidraw*; do
    echo "=== $dev ==="
    sudo udevadm info -q all -n $dev | grep -i "ID_VENDOR\|ID_MODEL"
done
```

## Limitations & Known Issues

### Current Limitations
1. **Linux only**: Uses hidraw, not portable to Windows/macOS
2. **Requires daemon for persistence**: Controller hardware resets LEDs after ~7 seconds
3. **Software-animated effects**: Effects are generated in software at 30 FPS (not hardware-accelerated)
4. **Manual port specification**: Doesn't auto-detect which ports have fans

### Implemented Features ✅
- ~~Solid colors only~~ ✅ **13 colors + custom RGB**
- ~~No effects~~ ✅ **7 animated effects**
- ~~No brightness~~ ✅ **0-100% brightness control**
- ~~No fan speed~~ ✅ **0-100% speed control**
- ~~No persistence~~ ✅ **Daemon mode with config files**
- ~~No status reading~~ ✅ **RPM and speed monitoring**

### Future Enhancements
- Additional effects (fire, sparkle, lightning)
- Music-reactive effects
- Temperature-based fan curves
- Per-LED custom patterns
- Web interface for live preview
- Windows/macOS support

## Technical Details

### Architecture
- **Single-file implementation**: All code in `src/main.rs`
- **Zero unsafe code**: Pure safe Rust
- **Minimal dependencies**: Only hidapi, clap, anyhow
- **Protocol-accurate**: Matches TTController C# behavior exactly

### Testing
Manual testing procedure:
1. Build the binary
2. Connect Riing Trio controller
3. Test init: Should complete without errors
4. Test OFF command: All LEDs should turn off
5. Test WHITE command: All LEDs should turn white
6. Test multiple ports: Repeat for ports 1-5
7. Test error cases: Try invalid port numbers, wrong PID

### Code Quality
- ✅ Clear variable names
- ✅ Protocol constants documented
- ✅ Error messages guide troubleshooting  
- ✅ Separation of concerns (HID transport, protocol, CLI)
- ✅ No magic numbers in logic

## Contributing

This is a minimal MVP implementation. For production use, consider:
- Adding more color options
- Implementing effect modes
- Adding configuration file support
- Creating a systemd service
- Port auto-detection

## References

- **TTController**: https://github.com/MoshiMoshi0/TTController (C# reference implementation)
- **Protocol**: Reverse-engineered from TTController source code
- **hidapi**: https://github.com/libusb/hidapi

## License

This implementation is provided as-is for educational and personal use.

## Credits

Protocol implementation based on TTController by MoshiMoshi0.
Rust implementation created as a minimal Linux MVP.
