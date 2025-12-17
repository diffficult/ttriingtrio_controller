# Daemon Mode Guide

## The Problem

The Thermaltake Riing Trio controller **resets itself after ~7 seconds** if it doesn't receive new commands. This means:
- LEDs revert to default rainbow mode
- Fan speeds revert to default settings
- Your changes don't persist

## The Solution: Daemon Mode

Daemon mode continuously reapplies your settings every 5 seconds (configurable), preventing the controller from resetting.

## Quick Start

### 1. Create Configuration File

```bash
# Copy the example config
cp riing-config.toml my-config.toml

# Edit it with your preferred settings
nano my-config.toml
```

Example configuration:
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

### 2. Run Daemon

```bash
# Run in foreground (see output)
sudo ./target/release/riing-trio-controller daemon --config my-config.toml

# Or specify custom interval
sudo ./target/release/riing-trio-controller daemon --config my-config.toml --interval 5
```

You'll see output like:
```
=== Riing Trio Controller - Daemon Mode ===
Device: 264a:2135
Config: my-config.toml
Interval: 5 seconds

✓ Configuration loaded
  Ports configured: 2
  Port 1:
    Speed: 50%
    Color: white
  Port 2:
    Speed: 75%
    Color: off

Starting daemon loop (Ctrl+C to stop)...
Applying settings every 5 seconds to prevent controller reset

[14:30:15] Applying settings...
  Port 1: Speed set to 50%
  Port 1: Color set to white
  Port 2: Speed set to 75%
  Port 2: Color set to off
✓ Settings applied (iteration 1)
```

### 3. Make It Permanent (Systemd Service)

To run automatically on boot:

```bash
# 1. Copy binary to system location
sudo cp target/release/riing-trio-controller /usr/local/bin/

# 2. Copy config to system location
sudo cp riing-config.toml /etc/riing-config.toml
sudo chmod 644 /etc/riing-config.toml

# 3. Install systemd service
sudo cp riing-trio-controller.service /etc/systemd/system/

# 4. Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable riing-trio-controller.service
sudo systemctl start riing-trio-controller.service

# 5. Check status
sudo systemctl status riing-trio-controller.service
```

View logs:
```bash
# Follow logs in real-time
sudo journalctl -u riing-trio-controller.service -f

# View recent logs
sudo journalctl -u riing-trio-controller.service -n 50
```

Stop/restart service:
```bash
sudo systemctl stop riing-trio-controller.service
sudo systemctl restart riing-trio-controller.service
```

## Configuration File Format

### Full Example

```toml
# Daemon settings
[daemon]
interval_seconds = 5    # Reapply settings every 5 seconds

# Configure each port with fans connected
[ports.1]
speed = 50              # Fan speed: 0-100%
color = "white"         # LED color: "off" or "white"
led_count = 30          # Number of LEDs (default: 30)

[ports.2]
speed = 75
color = "off"

[ports.3]
speed = 100
color = "white"
# led_count is optional, defaults to 30

# Ports without configuration will be ignored
```

### Settings Explained

**daemon.interval_seconds**
- How often to reapply settings
- Must be less than 7 seconds (controller reset time)
- Recommended: 5 seconds
- Lower = more USB traffic, higher = risk of reset

**ports.[N].speed**
- Fan speed percentage: 0-100
- 0 = minimum speed (fans still spin slightly)
- 100 = maximum speed
- Optional: omit if you don't want to control speed

**ports.[N].color**
- LED color to set
- Values: `"off"`, `"white"`
- Optional: omit if you don't want to control LEDs

**ports.[N].led_count**
- Number of LEDs on the fan
- Default: 30 (Riing Trio fans)
- Optional: only needed if using non-standard fans

## Testing Configuration

Before enabling the systemd service, test your configuration:

```bash
# Run daemon in foreground
sudo ./target/release/riing-trio-controller daemon --config riing-config.toml

# Watch it apply settings
# Press Ctrl+C when satisfied
```

If you see errors:
- Check config file syntax (valid TOML)
- Verify port numbers (1-5)
- Check speed values (0-100)
- Verify color values ("off" or "white")

## Troubleshooting

### Daemon Exits Immediately

Check logs:
```bash
sudo journalctl -u riing-trio-controller.service -n 50
```

Common issues:
- Config file not found: Check path in service file
- Permission denied: Service runs as root, but check file permissions
- Device not found: Ensure usbhid binding is set up (see HIDRAW_FIX.md)

### Settings Not Applying

1. Check daemon is running:
   ```bash
   sudo systemctl status riing-trio-controller.service
   ```

2. Verify config file location:
   ```bash
   cat /etc/riing-config.toml
   ```

3. Check daemon logs for errors:
   ```bash
   sudo journalctl -u riing-trio-controller.service -f
   ```

### Fans Still Reset to Rainbow

- Interval might be too long (>7 seconds)
- Daemon might have crashed (check logs)
- usbhid binding might not be persistent (check HIDRAW_FIX.md)

### High CPU Usage

Normal behavior: daemon wakes every 5 seconds to send commands.
CPU usage should be very low (< 1%).

If high:
- Check interval setting (too low?)
- Check for errors in logs (constant retries?)

## Advanced: Multiple Configurations

You can have different configs for different scenarios:

```bash
# Gaming setup (quiet fans, off LEDs)
sudo ./target/release/riing-trio-controller daemon --config gaming.toml

# Show setup (max fans, white LEDs)
sudo ./target/release/riing-trio-controller daemon --config show.toml

# Silent mode (minimum fans, off LEDs)
sudo ./target/release/riing-trio-controller daemon --config silent.toml
```

## Comparison: One-Time vs Daemon

**One-time command:**
```bash
sudo ./target/release/riing-trio-controller --port 1 off
# LEDs turn off... for 7 seconds, then back to rainbow
```

**Daemon mode:**
```bash
sudo ./target/release/riing-trio-controller daemon --config riing-config.toml
# LEDs stay off permanently (reapplied every 5 seconds)
```

## Why 7 Seconds?

The Thermaltake controller firmware has a built-in watchdog timer. If it doesn't receive HID commands for ~7 seconds, it assumes the controlling software crashed and reverts to default "rainbow" mode. This is by design to prevent fans from staying at unsafe speeds if the software fails.

Our daemon works around this by continuously "petting the watchdog" - sending commands every 5 seconds to keep the controller in our desired state.
