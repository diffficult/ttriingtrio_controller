# Time-Based Configuration System

This directory contains a complete time-based configuration rotation system using systemd timers.

## What's Included

### Configuration Files (4 configs for different times of day)

1. **sleep.toml** (3AM - 10AM)
   - All LEDs OFF
   - Fixed 50% fan speed on all ports
   - No temperature monitoring (save resources during sleep)

2. **work.toml** (10AM - 3:30PM)
   - All LEDs OFF
   - Temperature-reactive fan speeds (40% → 60% → 80% → 100%)
   - Port 1: CPU monitoring
   - Ports 2-3: GPU-NVIDIA monitoring

3. **evening.toml** (3:30PM - 8PM)
   - WAVE effect LEDs with temp-reactive colors:
     - Cyan (0-40°C)
     - Yellow (40-55°C)
     - Orange (55-75°C)
     - Red (75°C+)
   - Temperature-reactive fan speeds (40% → 60% → 80% → 100%)
   - Port 1: CPU monitoring
   - Ports 2-3: GPU-NVIDIA monitoring

4. **night.toml** (8PM - 3AM)
   - All LEDs OFF
   - Temperature-reactive fan speeds (40% → 60% → 80% → 100%)
   - Port 1: CPU monitoring
   - Ports 2-3: GPU-NVIDIA monitoring

### Systemd Files

**Service:**
- `riing-trio-controller.service` - Main daemon that reads `/etc/riing-trio/active.toml`

**Timer Services (4 switchers):**
- `riing-sleep.service` - Switches to sleep config
- `riing-work.service` - Switches to work config
- `riing-evening.service` - Switches to evening config
- `riing-night.service` - Switches to night config

**Timers (4 schedulers):**
- `riing-sleep.timer` - Triggers at 3:00 AM
- `riing-work.timer` - Triggers at 10:00 AM
- `riing-evening.timer` - Triggers at 3:30 PM (15:30)
- `riing-night.timer` - Triggers at 8:00 PM (20:00)

## How It Works

1. All 4 config files are installed to `/etc/riing-trio/`
2. A symlink `/etc/riing-trio/active.toml` points to the current config
3. The main daemon reads `active.toml` and applies those settings
4. Systemd timers trigger at scheduled times
5. Each timer service updates the symlink and restarts the daemon
6. The daemon picks up the new config and applies it

## Installation

See [INSTALL.md](INSTALL.md) for complete installation instructions.

**Quick Install:**
```bash
# Build project
cargo build --release

# Copy binary
sudo cp target/release/riing-trio-controller /usr/local/bin/

# Create config directory
sudo mkdir -p /etc/riing-trio

# Install configs
sudo cp systemd-configs/*.toml /etc/riing-trio/

# Set initial active config
sudo ln -sf /etc/riing-trio/work.toml /etc/riing-trio/active.toml

# Install systemd files
sudo cp systemd-configs/*.service /etc/systemd/system/
sudo cp systemd-configs/*.timer /etc/systemd/system/

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable --now riing-trio-controller.service
sudo systemctl enable --now riing-sleep.timer
sudo systemctl enable --now riing-work.timer
sudo systemctl enable --now riing-evening.timer
sudo systemctl enable --now riing-night.timer
```

## Visual Schedule

```
00:00 ─────────────────────────────────────────────────────────────
      │                                                           │
      │              NIGHT CONFIG (LEDs OFF)                     │
      │                                                           │
03:00 ─────────────────────────────────────────────────────────────
      │                                                           │
      │              SLEEP CONFIG (50% fixed)                    │
      │                                                           │
10:00 ─────────────────────────────────────────────────────────────
      │                                                           │
      │              WORK CONFIG (LEDs OFF)                      │
      │                                                           │
15:30 ─────────────────────────────────────────────────────────────
      │                                                           │
      │              EVENING CONFIG (WAVE LEDs)                  │
      │                                                           │
20:00 ─────────────────────────────────────────────────────────────
      │                                                           │
      │              NIGHT CONFIG (LEDs OFF)                     │
      │                                                           │
23:59 ─────────────────────────────────────────────────────────────
```

## Customization

### Change Schedule Times

Edit timer files:
```bash
sudo nano /etc/systemd/system/riing-evening.timer
```

Change `OnCalendar` line and reload:
```bash
sudo systemctl daemon-reload
sudo systemctl restart riing-evening.timer
```

### Modify Temperature Zones

Edit config files:
```bash
sudo nano /etc/riing-trio/evening.toml
```

Restart daemon to apply:
```bash
sudo systemctl restart riing-trio-controller.service
```

### Change Colors or Effects

Edit the config file you want to change:
```bash
sudo nano /etc/riing-trio/evening.toml
```

Available colors: cyan, yellow, orange, red, green, blue, magenta, purple, pink, white, off
Available effects: static, wave, pulse, spectrum, flow, ripple, blink

## Monitoring

```bash
# Check current config
ls -l /etc/riing-trio/active.toml

# Watch daemon logs
sudo journalctl -u riing-trio-controller.service -f

# See timer schedule
systemctl list-timers | grep riing

# Check specific timer
sudo systemctl status riing-evening.timer
```

## Manual Switching

Force a config change without waiting for the timer:

```bash
sudo systemctl start riing-evening.service  # Switch to evening
sudo systemctl start riing-work.service     # Switch to work
sudo systemctl start riing-sleep.service    # Switch to sleep
sudo systemctl start riing-night.service    # Switch to night
```

## Files in This Directory

```
systemd-configs/
├── README.md                          # This file
├── INSTALL.md                         # Detailed installation guide
├── sleep.toml                         # 3AM-10AM config
├── work.toml                          # 10AM-3:30PM config
├── evening.toml                       # 3:30PM-8PM config
├── night.toml                         # 8PM-3AM config
├── riing-trio-controller.service      # Main daemon service
├── riing-sleep.service                # Sleep switcher
├── riing-sleep.timer                  # 3AM trigger
├── riing-work.service                 # Work switcher
├── riing-work.timer                   # 10AM trigger
├── riing-evening.service              # Evening switcher
├── riing-evening.timer                # 3:30PM trigger
├── riing-night.service                # Night switcher
└── riing-night.timer                  # 8PM trigger
```
