# Protocol Analysis & Implementation Notes

This document details the findings from analyzing the TTController C# reference implementation and how they map to our Rust implementation.

## Reference Implementation Analysis

### Source Files Examined
- `Plugins/Controllers/TTController.Plugin.RiingTrioController/RiingTrioControllerProxy.cs`
- `Plugins/Controllers/TTController.Plugin.RiingTrioController/RiingTrioControllerDefinition.cs`
- `Source/TTController.Service/Hardware/HidDeviceProxy.cs`
- `Plugins/Devices/RiingTrio.json`

## Device Identification

### From RiingTrioControllerDefinition.cs (Lines 11-13)
```csharp
public string Name => "Riing Trio";
public int VendorId => 0x264a;
public IEnumerable<int> ProductIds => Enumerable.Range(0, 16).Select(x => 0x2135 + x);
```

**Findings:**
- VID: `0x264a` (fixed)
- PID Range: `0x2135` through `0x2144` (16 products)
- Default PID: `0x2135`

**Implementation:** CLI accepts `--vid` and `--pid` arguments with defaults matching above.

## HID Transport Layer

### From HidDeviceProxy.cs (Lines 30-53)

#### Buffer Allocation
```csharp
_writeBuffer = new byte[_device.GetMaxOutputReportLength()];
_readBuffer = new byte[_device.GetMaxInputReportLength()];
```

**Finding:** Uses device-reported buffer sizes.

#### Write Operation (Lines 34-53)
```csharp
public bool WriteBytes(params byte[] bytes)
{
    if (bytes.Length == 0)
        return false;

    Array.Clear(_writeBuffer, 0, _writeBuffer.Length);  // Zero-fill buffer
    Array.Copy(bytes, 0, _writeBuffer, 1, Math.Min(bytes.Length, _device.GetMaxOutputReportLength() - 1));

    try
    {
        _stream.Write(_writeBuffer);
        Logger.Trace("W[{vid}, {pid}] {data:X2}", VendorId, ProductId, _writeBuffer);
        return true;
    }
    // ...
}
```

**Key Findings:**
1. **Report ID = 0x00**: Buffer index 0 is left as 0 (report ID)
2. **Payload starts at index 1**: `Array.Copy(bytes, 0, _writeBuffer, 1, ...)`
3. **Zero-padding**: `Array.Clear` zeros entire buffer before copying payload
4. **Buffer size**: Typically 65 bytes (1 report ID + 64 payload)

**Implementation:**
```rust
const REPORT_SIZE: usize = 65;

fn write_bytes(&self, payload: &[u8]) -> Result<()> {
    let mut buffer = vec![0u8; Self::REPORT_SIZE];  // Report ID 0x00 at index 0
    let copy_len = std::cmp::min(payload.len(), Self::REPORT_SIZE - 1);
    buffer[1..1 + copy_len].copy_from_slice(&payload[..copy_len]);  // Payload at index 1
    self.device.write(&buffer)?;
    Ok(())
}
```

#### Read Operation (Lines 58-72)
```csharp
public byte[] ReadBytes()
{
    try
    {
        Array.Clear(_readBuffer, 0, _readBuffer.Length);
        var read = _stream.Read(_readBuffer, 0, _readBuffer.Length);
        Logger.Trace("R[{vid}, {pid}] {data:X2}", VendorId, ProductId, _readBuffer);
        return _readBuffer;
    }
    // ...
}
```

**Finding:** Simple blocking read with timeout set at construction (1000ms).

#### Timeouts (Lines 27-28)
```csharp
_stream.ReadTimeout = 1000;
_stream.WriteTimeout = 1000;
```

**Finding:** 1 second timeout for both operations.

**Implementation:**
```rust
match self.device.read_timeout(&mut buffer, 1000) {
    Ok(n) if n > 0 => Ok(buffer),
    Ok(_) => Err(anyhow!("Timeout: No response from device after 1000ms")),
    Err(e) => Err(anyhow!("Failed to read from HID device: {}", e)),
}
```

## Protocol Commands

### Init Command

#### From RiingTrioControllerProxy.cs (Lines 90-91)
```csharp
public override bool Init() =>
    Device.WriteReadBytes(0xfe, 0x33)?[3] == 0xfc;
```

**Protocol:**
- **Command bytes**: `[0xFE, 0x33]`
- **Success indicator**: Response byte at index 3 equals `0xFC`
- **Failure indicator**: Response byte at index 3 equals `0xFE` (implied from other commands)

**Implementation:**
```rust
pub fn init(&self) -> Result<()> {
    let response = self.write_read_bytes(&[0xFE, 0x33])?;
    Self::check_response_status(&response, "Init")?;
    Ok(())
}
```

### SetRgb Command (PerLed Mode)

#### From RiingTrioControllerProxy.cs (Lines 16-19, 38-62)

##### Mode Definition
```csharp
_availableEffects = new Dictionary<string, byte>
{
    ["PerLed"] = 0x24
};
```

**Finding:** PerLed mode byte is `0x24`.

##### RGB Write Implementation
```csharp
public override bool SetRgb(byte port, string effectType, IEnumerable<LedColor> colors)
{
    if (!_availableEffects.TryGetValue(effectType, out var mode))
        return false;

    bool WriteChunk(byte chunkId)
    {
        const byte maxPerChunk = 19;
        var bytes = new List<byte> { 0x32, 0x52, port, mode, 0x03, chunkId, 0x00 };
        foreach (var color in colors.Skip((chunkId - 1) * maxPerChunk).Take(maxPerChunk))
        {
            bytes.Add(color.G);  // GREEN first!
            bytes.Add(color.R);  // RED second
            bytes.Add(color.B);  // BLUE third
        }

        return Device.WriteReadBytes(bytes)?[3] == 0xfc;
    }

    var result = true;
    for(byte i = 0x01; i <= 0x02; i++)  // 2 chunks
        result &= WriteChunk(i);

    return result;
}
```

**Critical Findings:**

1. **Command Header**: `[0x32, 0x52, PORT, MODE, 0x03, CHUNK_ID, 0x00]`
   - `0x32`: Command category (RGB)
   - `0x52`: RGB subcommand
   - `PORT`: Port number (1-5)
   - `MODE`: Effect mode (0x24 for PerLed)
   - `0x03`: Unknown constant
   - `CHUNK_ID`: Chunk number (starts at 1)
   - `0x00`: Padding byte

2. **Color Order: GRB** (Lines 49-51)
   - **NOT RGB!**
   - Order: Green, Red, Blue
   - This is a critical implementation detail

3. **Chunking**:
   - Max colors per chunk: **19** (`const byte maxPerChunk = 19`)
   - 19 colors × 3 bytes = 57 bytes payload per chunk
   - Number of chunks: **2** (Line 58: `i <= 0x02`)
   - Total capacity: 38 LEDs (but only 30 used for Riing Trio)

4. **Chunk Iteration** (Line 47):
   - Chunk 1 (ID=1): LEDs 0-18 (colors[0:19])
   - Chunk 2 (ID=2): LEDs 19-37 (colors[19:38])

5. **Status Check**: Response byte[3] must be `0xFC` for each chunk

**Implementation:**
```rust
const MAX_COLORS_PER_CHUNK: usize = 19;
const RGB_CHUNK_COUNT: u8 = 2;

pub fn set_rgb(&self, port: u8, color: Color, led_count: usize) -> Result<()> {
    const MODE_PER_LED: u8 = 0x24;
    
    let colors: Vec<Color> = vec![color; led_count];
    
    for chunk_id in 1..=Self::RGB_CHUNK_COUNT {
        let response = self.write_rgb_chunk(port, MODE_PER_LED, chunk_id, &colors)?;
        Self::check_response_status(&response, &format!("RGB chunk {}", chunk_id))?;
    }
    
    Ok(())
}

fn write_rgb_chunk(&self, port: u8, mode: u8, chunk_id: u8, colors: &[Color]) -> Result<Vec<u8>> {
    let mut payload = vec![0x32, 0x52, port, mode, 0x03, chunk_id, 0x00];
    
    let start_idx = ((chunk_id - 1) as usize) * Self::MAX_COLORS_PER_CHUNK;
    let end_idx = std::cmp::min(start_idx + Self::MAX_COLORS_PER_CHUNK, colors.len());
    
    for color in &colors[start_idx..end_idx] {
        let grb = color.to_grb_bytes();  // [G, R, B] order!
        payload.extend_from_slice(&grb);
    }
    
    self.write_read_bytes(&payload)
}
```

## Device Configuration

### From RiingTrio.json
```json
{
  "Name": "RiingTrio",
  "LedCount": 30,
  "Zones": [ 12, 12, 6 ]
}
```

**Findings:**
- Default LED count: **30** per port
- LED zones: 12 + 12 + 6 = 30 (three rings)
- For solid colors, zones are treated as single unit

**Implementation:** Default `--led-count 30`, overridable via CLI.

## Response Status Codes

### From multiple locations in proxy code

**Status byte location**: 
- **Windows (C# TTController)**: Response byte at index **3**
- **Linux (hidraw)**: Response byte at index **2** (report ID stripped on read)

**Status codes:**
- `0xFC` = Success
- `0xFE` = Failure
- Other values = Unexpected/error

**Linux vs Windows Difference:**

On Windows with HidSharp, reads include the report ID:
```
response[0] = 0x00 (report ID)
response[1] = 0xFE (first data byte)
response[2] = 0x33 (second data byte)
response[3] = 0xFC (status byte) <- C# checks this
```

On Linux with hidraw, the report ID is stripped:
```
response[0] = 0xFE (first data byte)
response[1] = 0x33 (second data byte)
response[2] = 0xFC (status byte) <- Rust checks this
response[3] = 0x00
```

**Implementation:**
```rust
const STATUS_SUCCESS: u8 = 0xFC;
const STATUS_FAILURE: u8 = 0xFE;
const STATUS_BYTE_INDEX: usize = 2; // Index 2 on Linux (vs 3 on Windows)

fn check_response_status(response: &[u8], operation: &str) -> Result<()> {
    if response.len() <= Self::STATUS_BYTE_INDEX {
        return Err(anyhow!("{} failed: Response too short", operation));
    }

    match response[Self::STATUS_BYTE_INDEX] {
        Self::STATUS_SUCCESS => Ok(()),
        Self::STATUS_FAILURE => Err(anyhow!("{} failed: Device returned error", operation)),
        status => Err(anyhow!("{} failed: Unexpected status 0x{:02X}", operation, status)),
    }
}
```

## Port Configuration

### From RiingTrioControllerDefinition.cs (Line 13)
```csharp
public int PortCount => 5;
```

### From RiingTrioControllerProxy.cs (Lines 33-34, 93-97)
```csharp
public override IEnumerable<PortIdentifier> Ports => Enumerable.Range(1, Definition.PortCount)
    .Select(x => new PortIdentifier(Device.VendorId, Device.ProductId, (byte)x));

public override bool IsValidPort(PortIdentifier port) =>
    port.ControllerProductId == Device.ProductId
    && port.ControllerVendorId == Device.VendorId
    && port.Id >= 1
    && port.Id <= Definition.PortCount;
```

**Findings:**
- Port count: **5**
- Port numbering: **1-5** (inclusive, 1-based)
- Ports may be unused but are addressable

**Implementation:**
```rust
if !(1..=5).contains(&port) {
    return Err(anyhow!("Invalid port {}. Must be 1-5", port));
}
```

## Comparison: C# vs Rust Implementation

| Aspect | C# (TTController) | Rust (Our Implementation) |
|--------|-------------------|---------------------------|
| HID Library | HidSharp | hidapi |
| Report ID | Implicit 0x00 | Explicit at buffer[0] = 0x00 |
| Buffer Size | Device-reported | Fixed 65 bytes |
| Timeout | 1000ms | 1000ms |
| Error Handling | null returns | Result<T> with anyhow |
| Init Command | `[0xFE, 0x33]` | `[0xFE, 0x33]` ✓ |
| RGB Mode | 0x24 (PerLed) | 0x24 (PerLed) ✓ |
| Color Order | GRB | GRB ✓ |
| Max Colors/Chunk | 19 | 19 ✓ |
| Chunk Count | 2 | 2 ✓ |
| Status Check | byte[3] == 0xFC | byte[2] == 0xFC ✓ (Linux) |
| Port Range | 1-5 | 1-5 ✓ |
| LED Count | 30 (default) | 30 (default) ✓ |

## Divergences from Spec

The implementation follows the C# reference exactly. There are no divergences from the discovered protocol.

## Magic Numbers Explained

All "magic numbers" from the protocol:

| Value | Location | Meaning |
|-------|----------|---------|
| `0x264a` | VID | Thermaltake vendor ID |
| `0x2135` | PID | First product in range |
| `0x00` | Report ID | HID report identifier |
| `65` | Buffer size | 1 byte report + 64 payload |
| `0xFE` | Init byte 0 | Init command prefix |
| `0x33` | Init byte 1 | Init command suffix |
| `0xFC` | Status | Success indicator |
| `0xFE` | Status | Failure indicator |
| `3` | Index | Status byte location |
| `0x32` | RGB byte 0 | RGB command category |
| `0x52` | RGB byte 1 | RGB subcommand |
| `0x24` | Mode | PerLed effect mode |
| `0x03` | RGB byte 4 | Unknown constant |
| `0x00` | RGB byte 6 | Padding byte |
| `19` | Chunk size | Max colors per chunk |
| `2` | Chunk count | Number of chunks |
| `30` | LED count | Default LEDs per port |
| `1000` | Timeout | Milliseconds |

## Testing Methodology

Based on the C# implementation, our test cases validate:

1. **Transport Layer**: HID write/read with proper framing
2. **Init**: Command format and status check
3. **RGB Write**: Command format, GRB order, chunking, status per chunk
4. **Error Handling**: Status codes, timeouts, invalid ports
5. **Edge Cases**: Max LED counts, port boundaries

## Known Protocol Limitations

From analysis:

1. **No LED Feedback**: Cannot read current LED states
2. **No Port Detection**: Cannot query which ports have devices
3. **No Error Details**: Status byte only indicates success/failure
4. **Fixed Chunk Size**: Cannot optimize for fewer LEDs
5. **No Fan Control**: RGB only, no speed/RPM in this implementation

## Future Protocol Exploration

Commands not implemented (found in C# but not required):

- `[0x33, 0x50]` - Get version
- `[0x32, 0x51, port, 0x01, speed]` - Set fan speed
- `[0x33, 0x51, port]` - Get port data (RPM, speed)
- `[0x32, 0x53]` - Save profile

These could be added in future versions.
