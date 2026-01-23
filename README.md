[![Continuous Integration](https://img.shields.io/github/actions/workflow/status/yrakcaz/esp-layground/rust_ci.yml)](https://github.com/yrakcaz/esp-layground/actions)
[![MIT License](https://img.shields.io/github/license/yrakcaz/esp-layground?color=blue)](./LICENSE-MIT)

# ESPlayground

This project demonstrates the basic functionalities of the ESP32 microcontroller, specifically on the M5Stack Atom Lite development board. It serves as a playground for experimenting with various features of the ESP32, including GPIO, BLE, timers, LEDs, WiFi, and HTTP requests.

## Overview

The example in `main.rs` implements a simple state machine that integrates the following components:

- **Button Input**: A button is used to toggle the system state between "on" and "off."
- **BLE Scanner and Advertiser**: The system scans for nearby BLE devices and advertises its own state.
- **LED Control**: An LED is used to visually indicate the system state, with different colors and blinking patterns.
- **Timers**: Timers are used for periodic tasks, such as blinking the LED and scanning for BLE devices.
- **WiFi Connectivity**: The system connects to a specified WiFi network using credentials provided via environment variables.
- **HTTP Requests**: The system sends HTTP POST requests to a specified URL, but only if the "wifi" feature is enabled.

## Features

### WiFi Feature

The project includes a conditional "wifi" feature that enables or disables the WiFi and HTTP request mechanism. This is useful for scenarios where only one device in the system should handle HTTP requests (e.g., a server-client setup where the server handles requests).

To enable the "wifi" feature, you need to build or run the project with the `--features "wifi"` flag. For example:

```bash
cargo build --features "wifi"
cargo run --features "wifi"
```

When the "wifi" feature is enabled, the system initializes a WiFi connection and uses it to send HTTP requests. If the feature is disabled, the HTTP client is initialized without WiFi support.

## Environment Variables

The example requires the following environment variables to be set:

- `APP_NAME`: The name of the application.
- `WIFI_SSID`: The SSID of the WiFi network to connect to.
- `WIFI_PASSWORD`: The password for the WiFi network.
- `HTTP_URL`: The URL to which HTTP POST requests will be sent.

Ensure these variables are set in your environment before running the application.

## How It Works

1. The button toggles the system state between "on" and "off."
2. When the system is "on," the BLE scanner searches for nearby devices, and the LED blinks to indicate activity.
3. The BLE advertiser broadcasts the system's state.
4. A state machine coordinates the interactions between these components.

This example demonstrates how to use the ESP-IDF framework with Rust to build embedded applications for the ESP32 platform.
