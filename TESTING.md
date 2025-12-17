# Testing & Verification Guide

## Pre-Flight Checks

### 1. Verify Device Connection
```bash
# Check if device is connected
lsusb | grep 264a

# Expected output (example):
# Bus 001 Device 005: ID 264a:2135 Thermaltake ...
```

### 2. Check hidraw Access
```bash
# List hidraw devices
ls -l /dev/hidraw*

# Find which hidraw corresponds to Thermaltake
for dev in /dev/hidraw*; do
    echo "=== $dev ==="
    sudo udevadm info -q all -n $dev | grep -E "ID_VENDOR_ID|ID_MODEL_ID"
done

# Look for: ID_VENDOR_ID=264a
```

### 3. Build the Project
```bash
cd riing-trio-controller
cargo build --release

# Verify binary exists
ls -lh target/release/riing-trio-controller

# Check binary size (should be ~3-5 MB)
```

## Basic Functionality Tests

### Test 1: Help Command
```bash
./target/release/riing-trio-controller --help

# Expected: Usage information, no errors
```

### Test 2: Device Detection (with sudo if no udev rule)
```bash
# If udev rule not installed
sudo ./target/release/riing-trio-controller --port 1 off

# Expected: Device opens successfully
```

### Test 3: Init Sequence
```bash
# Run any command - init happens automatically
./target/release/riing-trio-controller --port 1 off

# Expected output:
# === Riing Trio RGB Controller ===
# Device: 264a:2135
# Port: 1
# LED Count: 30
# 
# Initializing controller...
# ✓ Controller initialized successfully
# ...
```

### Test 4: OFF Command
```bash
# Test each port
for port in 1 2 3 4 5; do
    echo "Testing port $port..."
    ./target/release/riing-trio-controller --port $port off
    sleep 1
done

# Expected: All LEDs on each port turn off
# Visual confirmation required
```

### Test 5: WHITE Command
```bash
# Test each port
for port in 1 2 3 4 5; do
    echo "Testing port $port..."
    ./target/release/riing-trio-controller --port $port white
    sleep 1
done

# Expected: All LEDs on each port turn white
# Visual confirmation required
```

### Test 6: Custom LED Count
```bash
# Test with different LED counts
./target/release/riing-trio-controller --port 1 --led-count 20 white
./target/release/riing-trio-controller --port 1 --led-count 30 white
./target/release/riing-trio-controller --port 1 --led-count 40 white

# Expected: Works without errors
# Note: Visual difference only if fewer LEDs physically present
```

### Test 7: Custom PID
```bash
# If your device uses a different PID
./target/release/riing-trio-controller --pid 0x2136 --port 1 off

# Expected: Opens device with specified PID
```

## Error Handling Tests

### Test 8: Invalid Port
```bash
./target/release/riing-trio-controller --port 0 off
./target/release/riing-trio-controller --port 6 off

# Expected: Clear error message "Invalid port"
```

### Test 9: Device Not Found
```bash
# Try with wrong PID
./target/release/riing-trio-controller --pid 0x9999 --port 1 off

# Expected: Error with troubleshooting hints
```

### Test 10: Permission Denied
```bash
# Only if udev rule NOT installed
# Run without sudo
./target/release/riing-trio-controller --port 1 off

# Expected: Permission error with hint to use sudo or install udev
```

## Protocol Validation Tests

### Test 11: Init Response Check
The init command should:
- Send: `[0xFE, 0x33]`
- Receive response with byte[3] == 0xFC
- Fail gracefully if byte[3] == 0xFE

### Test 12: RGB Chunking
For 30 LEDs:
- Should send 2 chunks
- Chunk 1: 19 colors (LEDs 1-19)
- Chunk 2: 11 colors (LEDs 20-30)
- Each chunk should receive 0xFC response

### Test 13: Color Order (GRB)
Visual inspection:
- WHITE command should produce pure white (not tinted)
- If colors appear wrong, check GRB order in code

## Performance Tests

### Test 14: Speed Test
```bash
# Time a single operation
time ./target/release/riing-trio-controller --port 1 off

# Expected: < 2 seconds total
# Breakdown:
# - Init: ~100-300ms
# - RGB write (2 chunks): ~200-400ms
```

### Test 15: Repeated Operations
```bash
# Test 10 iterations
for i in {1..10}; do
    ./target/release/riing-trio-controller --port 1 off
    ./target/release/riing-trio-controller --port 1 white
done

# Expected: No errors, consistent timing
```

## Integration Tests

### Test 16: Multi-Port Sequential
```bash
# Turn all ports white, then all off
./target/release/riing-trio-controller --port 1 white
./target/release/riing-trio-controller --port 2 white
./target/release/riing-trio-controller --port 3 white
./target/release/riing-trio-controller --port 4 white
./target/release/riing-trio-controller --port 5 white

sleep 2

./target/release/riing-trio-controller --port 1 off
./target/release/riing-trio-controller --port 2 off
./target/release/riing-trio-controller --port 3 off
./target/release/riing-trio-controller --port 4 off
./target/release/riing-trio-controller --port 5 off

# Expected: All ports respond correctly
```

### Test 17: Stress Test
```bash
# Rapid fire commands
for i in {1..50}; do
    ./target/release/riing-trio-controller --port 1 white > /dev/null
    ./target/release/riing-trio-controller --port 1 off > /dev/null
done

# Expected: No crashes, no USB disconnects
```

## Troubleshooting Validation

### Common Issues to Test

#### Issue: Device Not Found
```bash
# Verify with lsusb
lsusb -d 264a:

# If found but can't open, check permissions
ls -l /dev/hidraw*
```

#### Issue: Init Failure
```bash
# Try unplugging and replugging USB
# Run after reconnect
./target/release/riing-trio-controller --port 1 off
```

#### Issue: Timeout
```bash
# Check if device is accessible
cat /dev/hidraw* | hexdump -C | head

# Try different USB port
```

## Success Criteria

✅ **All tests must pass:**
1. Device opens successfully (with or without sudo)
2. Init returns success (0xFC)
3. OFF command turns LEDs off
4. WHITE command turns LEDs white
5. Multiple ports work independently
6. Error messages are clear and helpful
7. No crashes or hangs
8. Consistent performance

## Test Results Template

```
Test Date: ___________
Tester: ___________
Device PID: ___________

| Test | Status | Notes |
|------|--------|-------|
| 1. Help | ☐ Pass ☐ Fail | |
| 2. Device Detection | ☐ Pass ☐ Fail | |
| 3. Init Sequence | ☐ Pass ☐ Fail | |
| 4. OFF Command | ☐ Pass ☐ Fail | |
| 5. WHITE Command | ☐ Pass ☐ Fail | |
| 6. Custom LED Count | ☐ Pass ☐ Fail | |
| 7. Custom PID | ☐ Pass ☐ Fail | |
| 8. Invalid Port | ☐ Pass ☐ Fail | |
| 9. Device Not Found | ☐ Pass ☐ Fail | |
| 10. Permission Denied | ☐ Pass ☐ Fail | |
| 11. Init Response | ☐ Pass ☐ Fail | |
| 12. RGB Chunking | ☐ Pass ☐ Fail | |
| 13. Color Order | ☐ Pass ☐ Fail | |
| 14. Speed Test | ☐ Pass ☐ Fail | |
| 15. Repeated Ops | ☐ Pass ☐ Fail | |
| 16. Multi-Port | ☐ Pass ☐ Fail | |
| 17. Stress Test | ☐ Pass ☐ Fail | |

Overall Result: ☐ PASS ☐ FAIL
```

## Debug Mode

For detailed troubleshooting, you can add print statements to see:
- Exact bytes being sent
- Response bytes received
- Timing information

Modify `src/main.rs` to add debug output:
```rust
// In write_bytes function, add:
println!("DEBUG: Writing {} bytes: {:02x?}", buffer.len(), &buffer[..20]);

// In read_bytes function, add:
println!("DEBUG: Received {} bytes: {:02x?}", buffer.len(), &buffer[..20]);
```

Then rebuild and test:
```bash
cargo build --release
./target/release/riing-trio-controller --port 1 off
```
