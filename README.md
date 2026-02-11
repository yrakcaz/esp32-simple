[![Continuous Integration](https://img.shields.io/github/actions/workflow/status/yrakcaz/esp-layground/rust_ci.yml)](https://github.com/yrakcaz/esp-layground/actions)
[![MIT License](https://img.shields.io/github/license/yrakcaz/esp-layground?color=blue)](./LICENSE-MIT)

# ESPlayground

An ESP32 embedded development library and GPS tracking system built with Rust and ESP-IDF. This project provides reusable modules for BLE, WiFi, GPS, LED control, and more, along with client/server applications demonstrating a complete GPS tracking solution.

## Project Structure

This project consists of:
- **Library** - Reusable ESP32 modules for embedded development
- **Client Binary** - GPS tracker that advertises speed data via BLE
- **Server Binary** - BLE scanner that receives GPS data and posts to HTTP endpoint

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

## Applications

### Client (GPS Tracker)

The client application tracks GPS location and advertises maximum speed via BLE.

**Features:**
- Reads GPS data from UART sensor
- Calculates and tracks maximum speed
- Advertises speed data via BLE
- Button toggles tracking on/off
- LED indicates system state with colors and blinking patterns

**Usage:**
```bash
cargo run --bin client
# or (default binary):
cargo run
```

### Server (Data Receiver)

The server application scans for BLE devices, receives GPS data, and posts it to an HTTP endpoint.

**Features:**
- Scans for nearby BLE devices
- Receives speed data from client via BLE
- Connects to WiFi network
- Posts data to HTTP endpoint
- Button toggles scanning on/off
- LED indicates system state

**Usage:**
```bash
cargo run --bin server
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

### Optional (Both Binaries)
- `APP_NAME` - Application name (default: "ESPlayground")

### Required (Server Only)
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

cargo run --bin server
```

## Hardware

This project is designed for the **M5Stack Atom Lite** development board with ESP32 microcontroller.

**Required components:**
- M5Stack Atom Lite (ESP32-PICO)
- GPS sensor module (UART interface) - for client
- NeoPixel LED (built-in on M5Stack Atom Lite)
- Button (built-in on M5Stack Atom Lite)

## Development

### Building

```bash
# Build both binaries
cargo build --release

# Build specific binary
cargo build --release --bin client
cargo build --release --bin server
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
