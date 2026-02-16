[![Continuous Integration](https://img.shields.io/github/actions/workflow/status/yrakcaz/esp32-simple/rust_ci.yml)](https://github.com/yrakcaz/esp32-simple/actions)
[![Crates.io](https://img.shields.io/crates/v/esp32-simple)](https://crates.io/crates/esp32-simple)
[![Documentation](https://docs.rs/esp32-simple/badge.svg)](https://docs.rs/esp32-simple)
[![MIT License](https://img.shields.io/github/license/yrakcaz/esp32-simple?color=blue)](./LICENSE-MIT)

# esp32-simple

An ESP32 embedded development library and GPS tracking system built with Rust and ESP-IDF. This project provides reusable modules for BLE, WiFi, GPS, LED control, and more, along with client/server applications demonstrating a complete GPS tracking solution.

## Project Structure

This project consists of:
- **Library** - Reusable ESP32 modules for embedded development
- **Client Example** - GPS tracker that advertises speed data via BLE
- **Server Example** - BLE scanner that receives GPS data and posts to HTTP endpoint

## Using as a Library

Add `esp32-simple` to your ESP32 project:

```bash
cargo add esp32-simple
```

Or manually add to `Cargo.toml`:

```toml
[dependencies]
esp32-simple = "0.1"
```

Then import modules:

```rust
use esp32_simple::{
    ble::{Advertiser, Scanner},
    wifi::Connection,
    gps::Sensor,
    // ... other modules
};
```

## Library Modules

The library provides the following modules for ESP32 development:

- **`ble`** - Bluetooth Low Energy advertising and scanning
- **`button`** - Physical button input handling with polling-based debounce
- **`clock`** - Hardware timer management and interrupt configuration
- **`color`** - RGB color representation and predefined color constants
- **`gps`** - GPS sensor reading via UART and NMEA parsing
- **`http`** - HTTP client for sending requests over WiFi
- **`infra`** - Core infrastructure traits: `Poller`, `Switch`, and `State`
- **`light`** - NeoPixel LED control via the RMT peripheral
- **`message`** - Inter-thread messaging with triggers, notifiers, and dispatchers
- **`thread`** - Thread spawning with automatic device restart on failure
- **`time`** - Time utilities for sleeping and cooperative yielding
- **`wifi`** - WiFi connection management and configuration

## Examples

This crate includes two complete example applications demonstrating a GPS tracking system.

### Client Example (GPS Tracker)

The client example tracks GPS location and advertises maximum speed via BLE.

**Features:**
- Reads GPS data from UART sensor
- Calculates and tracks maximum speed
- Advertises speed data via BLE
- Button toggles tracking on/off
- LED indicates system state with colors and blinking patterns

**Usage:**
```bash
cargo run --example client
```

### Server Example (Data Receiver)

The server example scans for BLE devices, receives GPS data, and posts it to an HTTP endpoint.

**Features:**
- Scans for nearby BLE devices
- Receives speed data from client via BLE
- Connects to WiFi network
- Posts data to HTTP endpoint
- Button toggles scanning on/off
- LED indicates system state

**Usage:**
```bash
cargo run --example server
```

## Architecture

The system implements a client/server GPS tracking architecture:

1. **Client Device**:
   - GPS sensor continuously reads location data
   - Calculates maximum speed since tracking started
   - Advertises max speed as BLE manufacturer data
   - Button press resets max speed and toggles tracking

2. **Server Device**:
   - BLE scanner searches for client devices
   - Extracts speed data from BLE advertisements
   - Maintains WiFi connection
   - Posts received data to configured HTTP endpoint

3. **Communication**:
   - Client and server use BLE for wireless communication
   - Server uses WiFi for internet connectivity
   - Both use state machines to coordinate components

## Environment Variables

The following environment variables must be set at compile time:

### Optional (Both Examples)
- `APP_NAME` - Application name (default: "ESPlayground")

### Required (Server Example Only)
- `WIFI_SSID` - WiFi network SSID
- `WIFI_PASSWORD` - WiFi network password
- `HTTP_URL` - HTTP endpoint URL for posting data
- `HTTP_PARAM` - HTTP parameter name for the payload

Example:
```bash
export APP_NAME="MyESP32App"
export WIFI_SSID="MyNetwork"
export WIFI_PASSWORD="MyPassword"
export HTTP_URL="https://example.com/api/data"
export HTTP_PARAM="speed"

cargo run --example server
```

## Hardware

The **library** is ESP32 board-agnostic and can be used with any ESP32 development board.

The **examples** are designed for the **M5Stack Atom Lite** with specific hardware:

**Required components for examples:**
- M5Stack Atom Lite (ESP32-PICO) - base board for both examples
- Atomic GPS Base V2 (AT6668) - for client example

## Development

### Building

```bash
# Build library
cargo build --release

# Build examples
cargo build --release --examples

# Build specific example
cargo build --release --example client
cargo build --release --example server
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Generate documentation
cargo doc --no-deps --open
```

### Features

- `experimental` - Enables experimental features from `esp-idf-svc`

```bash
cargo build --features experimental
```

## How It Works

### Client Flow

1. GPS sensor thread continuously reads NMEA data from UART
2. When valid GPS reading is received, speed is calculated
3. Maximum speed is tracked and stored
4. Speed data is encoded and set as BLE manufacturer data
5. BLE advertiser broadcasts the current state and speed
6. LED blinks to indicate active GPS tracking
7. Button press toggles tracking and resets max speed

### Server Flow

1. BLE scanner periodically scans for nearby devices
2. When client device is detected, manufacturer data is extracted
3. Speed data is decoded from BLE payload
4. HTTP client posts the data to configured endpoint
5. LED indicates when active device is detected
6. Button press toggles scanning on/off

### State Machine

Both applications use a state machine pattern coordinating:
- Button input (toggle on/off)
- BLE operations (advertising/scanning)
- LED control (visual feedback)
- Timer-based periodic tasks
- Inter-thread messaging via FreeRTOS notifications

## License

This project is licensed under the MIT License. See [LICENSE-MIT](./LICENSE-MIT) for details.
