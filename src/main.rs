use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use hidapi::{HidApi, HidDevice};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Thermaltake Riing Trio RGB Controller
#[derive(Parser)]
#[command(name = "riing-trio-controller")]
#[command(about = "Control Thermaltake Riing Trio RGB LEDs and fan speed", long_about = None)]
struct Cli {
    /// USB Vendor ID (default: 0x264a)
    #[arg(long, default_value = "0x264a", value_parser = parse_hex)]
    vid: u16,

    /// USB Product ID (default: 0x2135, range: 0x2135-0x2144)
    #[arg(long, default_value = "0x2135", value_parser = parse_hex)]
    pid: u16,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Turn off all LEDs on the specified port
    Off {
        /// Port number (1-5)
        #[arg(short, long)]
        port: u8,

        /// Number of LEDs per port (default: 30 for Riing Trio)
        #[arg(long, default_value = "30")]
        led_count: usize,
    },
    
    /// Set all LEDs to white on the specified port
    White {
        /// Port number (1-5)
        #[arg(short, long)]
        port: u8,

        /// Number of LEDs per port (default: 30 for Riing Trio)
        #[arg(long, default_value = "30")]
        led_count: usize,
    },
    
    /// Set fan speed (0-100%)
    Speed {
        /// Port number (1-5)
        #[arg(short, long)]
        port: u8,

        /// Speed percentage (0-100)
        #[arg(short, long)]
        speed: u8,
    },
    
    /// Show current status (RPM, speed) for a port
    Status {
        /// Port number (1-5), or omit to show all ports
        #[arg(short, long)]
        port: Option<u8>,
    },
    
    /// Run as daemon, continuously applying settings from config file
    Daemon {
        /// Path to configuration file (default: ./riing-config.toml)
        #[arg(short, long, default_value = "riing-config.toml")]
        config: PathBuf,

        /// Interval in seconds between applying settings (default: 5)
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },
}

/// Parse hexadecimal string (with or without 0x prefix)
fn parse_hex(s: &str) -> Result<u16, std::num::ParseIntError> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u16::from_str_radix(s, 16)
}

/// Port status data (RPM, speed, etc.)
#[derive(Debug)]
struct PortStatus {
    _port_id: u8,  // Echoed port ID from device (not currently displayed)
    speed: u8,
    rpm: u16,
}

/// Configuration file structure
#[derive(Debug, Deserialize, Serialize)]
struct Config {
    #[serde(default)]
    ports: HashMap<String, PortConfig>,  // Changed from HashMap<u8, ...>
    
    #[serde(default)]
    daemon: DaemonConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct PortConfig {
    /// Fan speed (0-100)
    #[serde(default)]
    speed: Option<u8>,
    
    /// LED color: "off", "white", "red", "blue", etc. (for static mode)
    #[serde(default)]
    color: Option<String>,
    
    /// LED effect: "static", "spectrum", "wave", "pulse", "blink", "flow", "ripple"
    #[serde(default)]
    effect: Option<String>,
    
    /// Effect speed: "extreme", "fast", "normal", "slow"
    #[serde(default)]
    effect_speed: Option<String>,
    
    /// Flow effect colors (comma-separated)
    #[serde(default)]
    flow_colors: Option<String>,
    
    /// Brightness (0.0 to 1.0, default: 1.0)
    #[serde(default = "default_brightness")]
    brightness: f32,
    
    /// Number of LEDs (default: 30)
    #[serde(default = "default_led_count")]
    led_count: usize,
    
    /// Reapply speed in daemon mode (default: false, since speed persists)
    #[serde(default)]
    reapply_speed: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct DaemonConfig {
    /// Interval in seconds between applying settings
    #[serde(default = "default_interval")]
    interval_seconds: u64,
    
    /// Apply speed settings at startup only (recommended, since speed persists)
    #[serde(default = "default_true")]
    speed_once_at_startup: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 5,
            speed_once_at_startup: true,
        }
    }
}

fn default_led_count() -> usize {
    30
}

fn default_interval() -> u64 {
    5
}

fn default_true() -> bool {
    true
}

fn default_brightness() -> f32 {
    1.0
}

/// Parse effect from port configuration
fn parse_effect(port_config: &PortConfig) -> Result<Effect> {
    // If effect is specified, use it
    if let Some(ref effect_str) = port_config.effect {
        let speed = port_config.effect_speed
            .as_ref()
            .and_then(|s| EffectSpeed::from_str(s))
            .unwrap_or(EffectSpeed::Normal);
        
        match effect_str.to_lowercase().as_str() {
            "spectrum" | "rainbow" => {
                Ok(Effect::Spectrum { speed })
            }
            "wave" => {
                let color = port_config.color
                    .as_ref()
                    .and_then(|c| Color::from_str(c))
                    .unwrap_or(Color::BLUE);
                Ok(Effect::Wave { color, speed })
            }
            "pulse" | "breathing" => {
                let color = port_config.color
                    .as_ref()
                    .and_then(|c| Color::from_str(c))
                    .unwrap_or(Color::WHITE);
                Ok(Effect::Pulse { color, speed })
            }
            "blink" => {
                let color = port_config.color
                    .as_ref()
                    .and_then(|c| Color::from_str(c))
                    .unwrap_or(Color::WHITE);
                Ok(Effect::Blink { color, speed })
            }
            "flow" => {
                let colors = if let Some(ref flow_colors_str) = port_config.flow_colors {
                    flow_colors_str
                        .split(',')
                        .filter_map(|c| Color::from_str(c.trim()))
                        .collect::<Vec<_>>()
                } else {
                    vec![Color::RED, Color::GREEN, Color::BLUE]
                };
                
                if colors.is_empty() {
                    return Err(anyhow!("Flow effect requires at least one color"));
                }
                
                Ok(Effect::Flow { colors, speed })
            }
            "ripple" => {
                let color = port_config.color
                    .as_ref()
                    .and_then(|c| Color::from_str(c))
                    .unwrap_or(Color::CYAN);
                Ok(Effect::Ripple { color, speed })
            }
            "static" => {
                let color = port_config.color
                    .as_ref()
                    .and_then(|c| Color::from_str(c))
                    .unwrap_or(Color::WHITE);
                Ok(Effect::Static { color })
            }
            _ => Err(anyhow!("Unknown effect: {}", effect_str))
        }
    }
    // If only color is specified (no effect), use static
    else if let Some(ref color_str) = port_config.color {
        let color = Color::from_str(color_str)
            .ok_or_else(|| anyhow!("Unknown color: {}", color_str))?;
        Ok(Effect::Static { color })
    }
    else {
        Err(anyhow!("No effect or color specified"))
    }
}

/// Effect speed settings
#[derive(Debug, Clone, Copy)]
enum EffectSpeed {
    Extreme,  // Fastest
    Fast,
    Normal,
    Slow,
}

impl EffectSpeed {
    fn from_str(s: &str) -> Option<EffectSpeed> {
        match s.to_lowercase().as_str() {
            "extreme" => Some(EffectSpeed::Extreme),
            "fast" => Some(EffectSpeed::Fast),
            "normal" => Some(EffectSpeed::Normal),
            "slow" => Some(EffectSpeed::Slow),
            _ => None,
        }
    }
    
    /// Get frames per cycle (lower = faster)
    fn frames_per_cycle(&self) -> u32 {
        match self {
            EffectSpeed::Extreme => 30,   // 1 second at 30 FPS
            EffectSpeed::Fast => 60,      // 2 seconds
            EffectSpeed::Normal => 120,   // 4 seconds
            EffectSpeed::Slow => 240,     // 8 seconds
        }
    }
}

/// LED Effect types
#[derive(Debug, Clone)]
enum Effect {
    Static { color: Color },
    Spectrum { speed: EffectSpeed },
    Wave { color: Color, speed: EffectSpeed },
    Pulse { color: Color, speed: EffectSpeed },
    Blink { color: Color, speed: EffectSpeed },
    Flow { colors: Vec<Color>, speed: EffectSpeed },
    Ripple { color: Color, speed: EffectSpeed },
}

impl Effect {
    /// Generate LED colors for current frame
    fn generate(&self, frame: u32, led_count: usize, brightness: f32) -> Vec<Color> {
        match self {
            Effect::Static { color } => {
                vec![color.with_brightness(brightness); led_count]
            }
            
            Effect::Spectrum { speed } => {
                let cycle_frames = speed.frames_per_cycle();
                let hue_offset = (frame % cycle_frames) as f32 * 360.0 / cycle_frames as f32;
                
                (0..led_count)
                    .map(|_| Color::from_hsv(hue_offset, 1.0, 1.0).with_brightness(brightness))
                    .collect()
            }
            
            Effect::Wave { color, speed } => {
                let cycle_frames = speed.frames_per_cycle();
                let phase = (frame % cycle_frames) as f32 / cycle_frames as f32 * 2.0 * std::f32::consts::PI;
                
                (0..led_count)
                    .map(|i| {
                        let led_phase = phase + (i as f32 / led_count as f32) * 2.0 * std::f32::consts::PI;
                        let intensity = (led_phase.sin() * 0.5 + 0.5) * brightness;
                        color.with_brightness(intensity)
                    })
                    .collect()
            }
            
            Effect::Pulse { color, speed } => {
                let cycle_frames = speed.frames_per_cycle();
                let phase = (frame % cycle_frames) as f32 / cycle_frames as f32 * 2.0 * std::f32::consts::PI;
                let intensity = (phase.sin() * 0.5 + 0.5) * brightness;
                
                vec![color.with_brightness(intensity); led_count]
            }
            
            Effect::Blink { color, speed } => {
                let cycle_frames = speed.frames_per_cycle();
                let half_cycle = cycle_frames / 2;
                let is_on = (frame % cycle_frames) < half_cycle;
                
                if is_on {
                    vec![color.with_brightness(brightness); led_count]
                } else {
                    vec![Color::OFF; led_count]
                }
            }
            
            Effect::Flow { colors, speed } => {
                if colors.is_empty() {
                    return vec![Color::OFF; led_count];
                }
                
                let cycle_frames = speed.frames_per_cycle();
                let offset = (frame % cycle_frames) as f32 / cycle_frames as f32;
                
                (0..led_count)
                    .map(|i| {
                        let pos = (i as f32 / led_count as f32 + offset) % 1.0;
                        let color_idx = (pos * colors.len() as f32) as usize % colors.len();
                        colors[color_idx].with_brightness(brightness)
                    })
                    .collect()
            }
            
            Effect::Ripple { color, speed } => {
                let cycle_frames = speed.frames_per_cycle();
                let phase = (frame % cycle_frames) as f32 / cycle_frames as f32;
                
                (0..led_count)
                    .map(|i| {
                        let led_pos = i as f32 / led_count as f32;
                        let distance = (led_pos - 0.5).abs() * 2.0; // Distance from center
                        let wave = ((phase - distance) * std::f32::consts::PI * 2.0).sin();
                        let intensity = (wave * 0.5 + 0.5) * brightness;
                        color.with_brightness(intensity)
                    })
                    .collect()
            }
        }
    }
}

/// RGB color representation
#[derive(Debug, Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    // Basic colors
    const OFF: Color = Color { r: 0, g: 0, b: 0 };
    const WHITE: Color = Color { r: 255, g: 255, b: 255 };
    
    // Primary colors
    const RED: Color = Color { r: 255, g: 0, b: 0 };
    const GREEN: Color = Color { r: 0, g: 255, b: 0 };
    const BLUE: Color = Color { r: 0, g: 0, b: 255 };
    
    // Secondary colors
    const CYAN: Color = Color { r: 0, g: 255, b: 255 };
    const MAGENTA: Color = Color { r: 255, g: 0, b: 255 };
    const YELLOW: Color = Color { r: 255, g: 255, b: 0 };
    
    // Additional colors
    const ORANGE: Color = Color { r: 255, g: 165, b: 0 };
    const PURPLE: Color = Color { r: 128, g: 0, b: 128 };
    const PINK: Color = Color { r: 255, g: 192, b: 203 };
    const LIME: Color = Color { r: 0, g: 255, b: 0 };
    const SKY: Color = Color { r: 135, g: 206, b: 235 };

    /// Create custom color from RGB values
    fn from_rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }

    /// Convert to GRB byte order (as required by Riing Trio protocol)
    fn to_grb_bytes(&self) -> [u8; 3] {
        [self.g, self.r, self.b]
    }
    
    /// Parse color from string or RGB values
    fn from_str(s: &str) -> Option<Color> {
        match s.to_lowercase().as_str() {
            "off" | "black" => Some(Color::OFF),
            "white" => Some(Color::WHITE),
            "red" => Some(Color::RED),
            "green" => Some(Color::GREEN),
            "blue" => Some(Color::BLUE),
            "cyan" => Some(Color::CYAN),
            "magenta" => Some(Color::MAGENTA),
            "yellow" => Some(Color::YELLOW),
            "orange" => Some(Color::ORANGE),
            "purple" => Some(Color::PURPLE),
            "pink" => Some(Color::PINK),
            "lime" => Some(Color::LIME),
            "sky" => Some(Color::SKY),
            _ => None,
        }
    }
    
    /// Apply brightness (0.0 to 1.0)
    fn with_brightness(&self, brightness: f32) -> Color {
        let brightness = brightness.clamp(0.0, 1.0);
        Color {
            r: (self.r as f32 * brightness) as u8,
            g: (self.g as f32 * brightness) as u8,
            b: (self.b as f32 * brightness) as u8,
        }
    }
    
    /// Create color from HSV (Hue: 0-360, Saturation: 0-1, Value: 0-1)
    fn from_hsv(h: f32, s: f32, v: f32) -> Color {
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        let h = h % 360.0;
        
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        
        let (r, g, b) = match h as i32 {
            0..=59 => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=179 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        
        Color {
            r: ((r + m) * 255.0) as u8,
            g: ((g + m) * 255.0) as u8,
            b: ((b + m) * 255.0) as u8,
        }
    }
}

/// Riing Trio Controller
struct RiingTrioController {
    device: HidDevice,
}

impl RiingTrioController {
    /// Protocol constants from TTController C# implementation
    const REPORT_SIZE: usize = 65; // 1 byte report ID + 64 byte payload
    const MAX_COLORS_PER_CHUNK: usize = 19; // 19 colors * 3 bytes = 57 bytes
    const STATUS_SUCCESS: u8 = 0xFC;
    const STATUS_FAILURE: u8 = 0xFE;
    // NOTE: On Linux hidraw, the report ID is stripped on read, so status is at index 2 (not 3 like on Windows)
    const STATUS_BYTE_INDEX: usize = 2; // response[2] contains status on Linux
    const RGB_CHUNK_COUNT: u8 = 2; // Riing Trio uses 2 chunks (30 LEDs fits in 38 slots)

    /// Open HID device by VID/PID
    fn open(vid: u16, pid: u16) -> Result<Self> {
        let api = HidApi::new().context("Failed to initialize HID API")?;

        let device = api
            .open(vid, pid)
            .with_context(|| format!("Failed to open HID device {:04x}:{:04x}", vid, pid))
            .map_err(|e| {
                anyhow!(
                    "{}\n\nTroubleshooting:\n\
                     - Ensure device is connected\n\
                     - Check if you need root/sudo access\n\
                     - Try creating a udev rule (see README)\n\
                     - Verify VID:PID with 'lsusb' command",
                    e
                )
            })?;

        // Set read timeout to 1000ms (matching C# implementation)
        device
            .set_blocking_mode(true)
            .context("Failed to set blocking mode")?;

        Ok(Self { device })
    }

    /// Write HID report with proper framing
    /// 
    /// Protocol: [Report-ID=0x00][Payload bytes...][Zero padding to REPORT_SIZE]
    /// 
    /// The C# implementation:
    /// - Sets byte 0 to 0x00 (report ID)
    /// - Copies payload starting at byte 1
    /// - Zero-pads the rest
    fn write_bytes(&self, payload: &[u8]) -> Result<()> {
        let mut buffer = vec![0u8; Self::REPORT_SIZE];
        
        // Report ID is 0x00 (already set by initialization)
        // Copy payload starting at byte 1
        let copy_len = std::cmp::min(payload.len(), Self::REPORT_SIZE - 1);
        buffer[1..1 + copy_len].copy_from_slice(&payload[..copy_len]);

        self.device
            .write(&buffer)
            .context("Failed to write to HID device")?;

        Ok(())
    }

    /// Read HID report
    fn read_bytes(&self) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; Self::REPORT_SIZE];
        
        // Use a timeout (hidapi handles this internally with blocking mode)
        match self.device.read_timeout(&mut buffer, 1000) {
            Ok(n) if n > 0 => Ok(buffer),
            Ok(_) => Err(anyhow!("Timeout: No response from device after 1000ms")),
            Err(e) => Err(anyhow!("Failed to read from HID device: {}", e)),
        }
    }

    /// Write command and read response
    fn write_read_bytes(&self, payload: &[u8]) -> Result<Vec<u8>> {
        self.write_bytes(payload)?;
        self.read_bytes()
    }

    /// Check if response indicates success
    /// 
    /// From C# code: response[3] == 0xFC means success (on Windows)
    /// On Linux hidraw: response[2] == 0xFC (report ID is stripped)
    /// response[2] == 0xFE means failure
    fn check_response_status(response: &[u8], operation: &str) -> Result<()> {
        if response.len() <= Self::STATUS_BYTE_INDEX {
            return Err(anyhow!(
                "{} failed: Response too short ({} bytes)",
                operation,
                response.len()
            ));
        }

        match response[Self::STATUS_BYTE_INDEX] {
            Self::STATUS_SUCCESS => Ok(()),
            Self::STATUS_FAILURE => Err(anyhow!("{} failed: Device returned error (0xFE)", operation)),
            status => Err(anyhow!(
                "{} failed: Unexpected status 0x{:02X} (expected 0xFC)",
                operation,
                status
            )),
        }
    }

    /// Initialize controller
    /// 
    /// Command: [0xFE, 0x33]
    /// Success: response[3] == 0xFC
    pub fn init(&self) -> Result<()> {
        println!("Initializing controller...");
        
        let response = self
            .write_read_bytes(&[0xFE, 0x33])
            .context("Init command failed")?;
        
        Self::check_response_status(&response, "Init")?;
        
        println!("✓ Controller initialized successfully");
        Ok(())
    }

    /// Set RGB color for all LEDs on a port
    /// 
    /// Command format: [0x32, 0x52, PORT, MODE, 0x03, CHUNK_ID, 0x00, G, R, B, ...]
    /// 
    /// Important protocol details from C# implementation:
    /// - MODE = 0x24 for PerLed effect
    /// - Colors are in GRB order (NOT RGB!)
    /// - Max 19 colors per chunk
    /// - Riing Trio uses 2 chunks (CHUNK_ID: 1, 2)
    /// - Each chunk must receive success response (0xFC) before sending next
    pub fn set_rgb(&self, port: u8, color: Color, led_count: usize) -> Result<()> {
        let colors = vec![color; led_count];
        self.set_rgb_colors(port, &colors)
    }
    
    /// Set RGB colors from a pre-generated color array (for effects)
    pub fn set_rgb_colors(&self, port: u8, colors: &[Color]) -> Result<()> {
        const MODE_PER_LED: u8 = 0x24;

        // Validate port
        if !(1..=5).contains(&port) {
            return Err(anyhow!("Invalid port {}. Must be 1-5", port));
        }

        // Send colors in chunks
        for chunk_id in 1..=Self::RGB_CHUNK_COUNT {
            let chunk_result = self.write_rgb_chunk(port, MODE_PER_LED, chunk_id, colors)?;
            
            Self::check_response_status(
                &chunk_result,
                &format!("RGB write chunk {}/{}", chunk_id, Self::RGB_CHUNK_COUNT)
            )?;
        }

        Ok(())
    }

    /// Set fan speed for a port
    /// 
    /// Command format: [0x32, 0x51, PORT, 0x01, SPEED]
    /// 
    /// - SPEED: 0-100 (percentage)
    /// - Response: Check byte[2] == 0xFC for success
    pub fn set_speed(&self, port: u8, speed: u8) -> Result<()> {
        // Validate port
        if !(1..=5).contains(&port) {
            return Err(anyhow!("Invalid port {}. Must be 1-5", port));
        }

        // Validate speed
        if speed > 100 {
            return Err(anyhow!("Invalid speed {}. Must be 0-100", speed));
        }

        let response = self
            .write_read_bytes(&[0x32, 0x51, port, 0x01, speed])
            .context("Set speed command failed")?;

        Self::check_response_status(&response, "Set speed")?;

        Ok(())
    }

    /// Get port status (RPM, speed, etc.)
    /// 
    /// Command format: [0x33, 0x51, PORT]
    /// 
    /// Response format (Linux, report ID stripped):
    /// - byte[0]: 0x33 (echo of command)
    /// - byte[1]: 0x51 (echo of subcommand)
    /// - byte[2]: port_id (0xFC = success, 0xFE = failure)
    /// - byte[3]: unknown
    /// - byte[4]: speed (0-100)
    /// - byte[5]: RPM low byte
    /// - byte[6]: RPM high byte
    pub fn get_port_status(&self, port: u8) -> Result<PortStatus> {
        // Validate port
        if !(1..=5).contains(&port) {
            return Err(anyhow!("Invalid port {}. Must be 1-5", port));
        }

        let response = self
            .write_read_bytes(&[0x33, 0x51, port])
            .context("Get port status command failed")?;

        // Check if port has a device (0xFE = no device)
        if response.len() > 2 && response[2] == 0xFE {
            return Err(anyhow!("No device connected on port {}", port));
        }

        // Parse response
        if response.len() < 7 {
            return Err(anyhow!("Invalid response length: {}", response.len()));
        }

        let port_id = response[2];
        let speed = response[4];
        let rpm_low = response[5] as u16;
        let rpm_high = response[6] as u16;
        let rpm = (rpm_high << 8) | rpm_low;

        Ok(PortStatus {
            _port_id: port_id,
            speed,
            rpm,
        })
    }

    /// Write a single RGB chunk
    /// 
    /// Chunk format: [0x32, 0x52, PORT, MODE, 0x03, CHUNK_ID, 0x00, COLORS...]
    /// 
    /// COLORS are in GRB order: [G1, R1, B1, G2, R2, B2, ...]
    /// Max 19 colors per chunk (19 * 3 = 57 bytes)
    fn write_rgb_chunk(
        &self,
        port: u8,
        mode: u8,
        chunk_id: u8,
        colors: &[Color],
    ) -> Result<Vec<u8>> {
        let mut payload = vec![0x32, 0x52, port, mode, 0x03, chunk_id, 0x00];

        // Calculate which colors belong to this chunk
        let start_idx = ((chunk_id - 1) as usize) * Self::MAX_COLORS_PER_CHUNK;
        let end_idx = std::cmp::min(start_idx + Self::MAX_COLORS_PER_CHUNK, colors.len());

        // Add colors in GRB order
        for color in &colors[start_idx..end_idx] {
            let grb = color.to_grb_bytes();
            payload.extend_from_slice(&grb);
        }

        // Send chunk and read response
        self.write_read_bytes(&payload)
            .with_context(|| format!("Failed to write RGB chunk {}", chunk_id))
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon { config, interval } => {
            run_daemon(cli.vid, cli.pid, config, interval)
        }
        _ => {
            // Single command mode
            run_single_command(cli)
        }
    }
}

fn run_single_command(cli: Cli) -> Result<()> {
    println!("\n=== Riing Trio RGB Controller ===");
    println!("Device: {:04x}:{:04x}", cli.vid, cli.pid);
    println!();

    // Open device
    let controller = RiingTrioController::open(cli.vid, cli.pid)?;

    // Initialize
    println!("Initializing controller...");
    controller.init()?;
    println!("✓ Controller initialized successfully\n");

    // Execute command
    match cli.command {
        Commands::Off { port, led_count } => {
            println!("Turning off LEDs on port {}...", port);
            controller.set_rgb(port, Color::OFF, led_count)?;
            println!("✓ LEDs turned off on port {}", port);
        }
        
        Commands::White { port, led_count } => {
            println!("Setting LEDs to white on port {}...", port);
            controller.set_rgb(port, Color::WHITE, led_count)?;
            println!("✓ LEDs set to white on port {}", port);
        }
        
        Commands::Speed { port, speed } => {
            println!("Setting fan speed to {}% on port {}...", speed, port);
            controller.set_speed(port, speed)?;
            println!("✓ Fan speed set to {}% on port {}", speed, port);
        }
        
        Commands::Status { port } => {
            if let Some(p) = port {
                // Single port status
                match controller.get_port_status(p) {
                    Ok(status) => {
                        println!("Port {} Status:", p);
                        println!("  Speed: {}%", status.speed);
                        println!("  RPM: {}", status.rpm);
                    }
                    Err(e) => {
                        println!("Port {}: {}", p, e);
                    }
                }
            } else {
                // All ports status
                println!("Scanning all ports...\n");
                for p in 1..=5 {
                    match controller.get_port_status(p) {
                        Ok(status) => {
                            println!("Port {}:", p);
                            println!("  Speed: {}%", status.speed);
                            println!("  RPM: {}", status.rpm);
                            println!();
                        }
                        Err(e) => {
                            println!("Port {}: {}\n", p, e);
                        }
                    }
                }
            }
        }
        
        Commands::Daemon { .. } => unreachable!(),
    }

    println!("\n✓ Operation completed successfully!\n");
    Ok(())
}

fn run_daemon(vid: u16, pid: u16, config_path: PathBuf, interval: u64) -> Result<()> {
    println!("\n=== Riing Trio Controller - Daemon Mode ===");
    println!("Device: {:04x}:{:04x}", vid, pid);
    println!("Config: {}", config_path.display());
    println!();

    // Load configuration
    let config = load_config(&config_path)?;
    println!("✓ Configuration loaded");
    println!("  Ports configured: {}", config.ports.len());
    
    // Parse effects for each port
    let mut port_effects: HashMap<u8, Effect> = HashMap::new();
    let mut port_brightness: HashMap<u8, f32> = HashMap::new();
    let mut port_led_counts: HashMap<u8, usize> = HashMap::new();
    let mut has_animated_effects = false;
    
    for (port_str, port_config) in &config.ports {
        let port: u8 = port_str.parse()
            .with_context(|| format!("Invalid port number: {}", port_str))?;
        
        println!("  Port {}:", port);
        if let Some(speed) = port_config.speed {
            println!("    Speed: {}%", speed);
        }
        
        match parse_effect(port_config) {
            Ok(effect) => {
                let effect_name = match &effect {
                    Effect::Static { .. } => "static",
                    Effect::Spectrum { .. } => "spectrum",
                    Effect::Wave { .. } => "wave",
                    Effect::Pulse { .. } => "pulse",
                    Effect::Blink { .. } => "blink",
                    Effect::Flow { .. } => "flow",
                    Effect::Ripple { .. } => "ripple",
                };
                
                println!("    Effect: {}", effect_name);
                if port_config.brightness < 1.0 {
                    println!("    Brightness: {:.0}%", port_config.brightness * 100.0);
                }
                
                if !matches!(effect, Effect::Static { .. }) {
                    has_animated_effects = true;
                }
                
                port_effects.insert(port, effect);
                port_brightness.insert(port, port_config.brightness);
                port_led_counts.insert(port, port_config.led_count);
            }
            Err(e) => {
                eprintln!("    Error: {}", e);
            }
        }
    }
    
    let speed_once = config.daemon.speed_once_at_startup;
    if speed_once {
        println!("\n✓ Fan speed will be set once at startup (speeds persist)");
    }
    
    if has_animated_effects {
        println!("✓ Animated effects will run at 30 FPS");
    } else {
        println!("✓ Static LEDs will be reapplied every {} seconds (LEDs reset)", interval);
    }
    println!();

    // Open device
    let controller = RiingTrioController::open(vid, pid)?;
    
    // Initialize
    println!("Initializing controller...");
    controller.init()?;
    println!("✓ Controller initialized\n");

    // Apply speed settings once at startup if configured
    if speed_once {
        println!("Setting fan speeds (one-time)...");
        for (port_str, port_config) in &config.ports {
            let port: u8 = port_str.parse()
                .with_context(|| format!("Invalid port number: {}", port_str))?;
            
            if let Some(speed) = port_config.speed {
                match controller.set_speed(port, speed) {
                    Ok(_) => println!("  Port {}: Speed set to {}%", port, speed),
                    Err(e) => eprintln!("  Port {}: Failed to set speed: {}", port, e),
                }
            }
        }
        println!("✓ Fan speeds configured\n");
    }

    println!("Starting daemon loop (Ctrl+C to stop)...\n");

    // Determine update interval based on effects
    let frame_duration = if has_animated_effects {
        Duration::from_millis(33) // ~30 FPS
    } else {
        Duration::from_secs(interval) // Static colors at configured interval
    };
    
    let mut frame: u32 = 0;
    let mut last_speed_apply = std::time::Instant::now();
    let speed_interval = Duration::from_secs(interval);

    loop {
        let loop_start = std::time::Instant::now();
        
        // Show periodic status (every 5 seconds for animated, every iteration for static)
        let should_log = if has_animated_effects {
            frame % 150 == 0 // Every 5 seconds at 30 FPS
        } else {
            true
        };
        
        if should_log {
            println!("[{}] Applying settings (frame {})...", 
                chrono::Local::now().format("%H:%M:%S"), frame);
        }

        for (port_str, port_config) in &config.ports {
            let port: u8 = match port_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };
            
            // Apply speed if needed
            if let Some(speed) = port_config.speed {
                let should_apply_speed = !speed_once || 
                    port_config.reapply_speed || 
                    last_speed_apply.elapsed() >= speed_interval;
                
                if should_apply_speed && (!has_animated_effects || frame % 150 == 0) {
                    if let Err(e) = controller.set_speed(port, speed) {
                        if should_log {
                            eprintln!("  Port {}: Failed to set speed: {}", port, e);
                        }
                    }
                }
            }

            // Apply LED effect
            if let Some(effect) = port_effects.get(&port) {
                let brightness = *port_brightness.get(&port).unwrap_or(&1.0);
                let led_count = *port_led_counts.get(&port).unwrap_or(&30);
                
                let colors = effect.generate(frame, led_count, brightness);
                
                // Send colors to controller
                if let Err(e) = controller.set_rgb_colors(port, &colors) {
                    if should_log {
                        eprintln!("  Port {}: Failed to set LEDs: {}", port, e);
                    }
                }
            }
        }

        if should_log {
            println!("✓ Settings applied\n");
        }
        
        if frame % 150 == 0 {
            last_speed_apply = std::time::Instant::now();
        }

        frame = frame.wrapping_add(1);

        // Sleep for remaining time to maintain FPS
        let elapsed = loop_start.elapsed();
        if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
        }
    }
}

fn load_config(path: &PathBuf) -> Result<Config> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    
    let config: Config = toml::from_str(&contents)
        .context("Failed to parse config file")?;
    
    Ok(config)
}
