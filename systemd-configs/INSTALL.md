# Time-Based Configuration Rotation - Installation Guide

This setup automatically rotates between 4 different configurations throughout the day using systemd timers.

## Schedule Overview

| Time | Config | Description |
|------|--------|-------------|
| **3:00 AM - 10:00 AM** | `sleep.toml` | LEDs OFF, 50% fixed speed (sleep hours) |
| **10:00 AM - 3:30 PM** | `work.toml` | LEDs OFF, temp-reactive speeds (work hours) |
| **3:30 PM - 8:00 PM** | `evening.toml` | WAVE effects, temp-reactive colors & speeds (evening) |
| **8:00 PM - 3:00 AM** | `night.toml` | LEDs OFF, temp-reactive speeds (night hours) |

## Configuration Details

### Port Setup
- **Port 1** (Top of case): CPU temperature monitoring
- **Ports 2 & 3** (Front of case): GPU-NVIDIA temperature monitoring

### Temperature Zones (All Configs Except Sleep)
| Temp Range | Fan Speed | LED Effect (evening only) |
|------------|-----------|---------------------------|
| 0-40°C | 40% | Wave - Cyan |
| 40-55°C | 60% | Wave - Yellow |
| 55-75°C | 80% | Wave - Orange |
| 75°C+ | 100% | Wave - Red (fast) |

## Installation Steps

### 1. Build the Project (if not already done)

```bash
cd /home/rx/dev/mygits/ttriingtrio_controller
cargo build --release
```

### 2. Install the Binary

```bash
sudo cp target/release/riing-trio-controller /usr/local/bin/
sudo chmod +x /usr/local/bin/riing-trio-controller
```

### 3. Create Configuration Directory

```bash
sudo mkdir -p /etc/riing-trio
```

### 4. Install Configuration Files

```bash
sudo cp systemd-configs/sleep.toml /etc/riing-trio/
sudo cp systemd-configs/work.toml /etc/riing-trio/
sudo cp systemd-configs/evening.toml /etc/riing-trio/
sudo cp systemd-configs/night.toml /etc/riing-trio/
```

### 5. Set Initial Active Configuration

Choose which config to start with based on current time, or use work config as default:

```bash
sudo ln -sf /etc/riing-trio/work.toml /etc/riing-trio/active.toml
```

### 6. Install Systemd Service and Timers

```bash
# Install main service
sudo cp systemd-configs/riing-trio-controller.service /etc/systemd/system/

# Install timer services and timers
sudo cp systemd-configs/riing-sleep.service /etc/systemd/system/
sudo cp systemd-configs/riing-sleep.timer /etc/systemd/system/

sudo cp systemd-configs/riing-work.service /etc/systemd/system/
sudo cp systemd-configs/riing-work.timer /etc/systemd/system/

sudo cp systemd-configs/riing-evening.service /etc/systemd/system/
sudo cp systemd-configs/riing-evening.timer /etc/systemd/system/

sudo cp systemd-configs/riing-night.service /etc/systemd/system/
sudo cp systemd-configs/riing-night.timer /etc/systemd/system/
```

### 7. Reload Systemd and Enable Services

```bash
# Reload systemd to recognize new units
sudo systemctl daemon-reload

# Enable and start the main daemon
sudo systemctl enable riing-trio-controller.service
sudo systemctl start riing-trio-controller.service

# Enable all timers (they will trigger at their scheduled times)
sudo systemctl enable riing-sleep.timer
sudo systemctl enable riing-work.timer
sudo systemctl enable riing-evening.timer
sudo systemctl enable riing-night.timer

# Start all timers
sudo systemctl start riing-sleep.timer
sudo systemctl start riing-work.timer
sudo systemctl start riing-evening.timer
sudo systemctl start riing-night.timer
```

## Verification

### Check Main Service Status

```bash
sudo systemctl status riing-trio-controller.service
```

### Check Which Config is Active

```bash
ls -l /etc/riing-trio/active.toml
# Should show which config file is currently linked
```

### Check Timer Status

```bash
# List all timers and see when they'll trigger next
systemctl list-timers | grep riing

# Check individual timer
sudo systemctl status riing-work.timer
sudo systemctl status riing-evening.timer
sudo systemctl status riing-sleep.timer
sudo systemctl status riing-night.timer
```

### View Logs

```bash
# Watch live logs from main daemon
sudo journalctl -u riing-trio-controller.service -f

# View last 50 lines
sudo journalctl -u riing-trio-controller.service -n 50

# Check timer activation logs
sudo journalctl -u riing-work.service -n 20
```

## Manual Config Switching

You can manually switch configs at any time:

```bash
# Switch to sleep config
sudo systemctl start riing-sleep.service

# Switch to work config
sudo systemctl start riing-work.service

# Switch to evening config
sudo systemctl start riing-evening.service

# Switch to night config
sudo systemctl start riing-night.service
```

The timer will override your manual change at the next scheduled time.

## Customization

### Modify Schedule Times

Edit the timer files in `/etc/systemd/system/`:

```bash
sudo nano /etc/systemd/system/riing-work.timer
```

Change the `OnCalendar` line:
```ini
OnCalendar=*-*-* 10:00:00  # Format: YYYY-MM-DD HH:MM:SS
```

After editing, reload and restart:
```bash
sudo systemctl daemon-reload
sudo systemctl restart riing-work.timer
```

### Modify Temperature Zones or Colors

Edit config files in `/etc/riing-trio/`:

```bash
sudo nano /etc/riing-trio/evening.toml
```

After editing, restart the daemon:
```bash
sudo systemctl restart riing-trio-controller.service
```

## Troubleshooting

### Daemon Won't Start

```bash
# Check if device is connected
lsusb | grep 264a

# Check if driver is bound
ls /dev/hidraw*

# Check service status
sudo systemctl status riing-trio-controller.service

# View full logs
sudo journalctl -u riing-trio-controller.service -n 100
```

### Timers Not Triggering

```bash
# Verify timers are enabled and active
systemctl list-timers --all | grep riing

# Check timer status
sudo systemctl status riing-work.timer

# View timer logs
sudo journalctl -u riing-work.timer -n 20
```

### Config Not Switching

```bash
# Check if symlink exists and is correct
ls -la /etc/riing-trio/active.toml

# Manually trigger a switch to test
sudo systemctl start riing-evening.service

# Check switch service logs
sudo journalctl -u riing-evening.service -n 20
```

### NVIDIA GPU Temp Not Reading

```bash
# Test nvidia-smi manually
nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits

# If it fails, install NVIDIA drivers
```

## Uninstallation

```bash
# Stop and disable all services
sudo systemctl stop riing-trio-controller.service
sudo systemctl disable riing-trio-controller.service

sudo systemctl stop riing-*.timer
sudo systemctl disable riing-sleep.timer
sudo systemctl disable riing-work.timer
sudo systemctl disable riing-evening.timer
sudo systemctl disable riing-night.timer

# Remove service files
sudo rm /etc/systemd/system/riing-trio-controller.service
sudo rm /etc/systemd/system/riing-*.service
sudo rm /etc/systemd/system/riing-*.timer

# Reload systemd
sudo systemctl daemon-reload

# Optionally remove configs (WARNING: deletes your customizations!)
# sudo rm -rf /etc/riing-trio

# Optionally remove binary
# sudo rm /usr/local/bin/riing-trio-controller
```

## Quick Reference

```bash
# Check what's running now
sudo systemctl status riing-trio-controller.service
ls -l /etc/riing-trio/active.toml

# View live logs
sudo journalctl -u riing-trio-controller.service -f

# See when next config switch happens
systemctl list-timers | grep riing

# Manually switch config
sudo systemctl start riing-evening.service

# Restart daemon after config edit
sudo systemctl restart riing-trio-controller.service
```
