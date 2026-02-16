#![feature(never_type)]

//! ESP32 embedded development library providing BLE, Wi-Fi, HTTP, GPS, LED,
//! button, and timer functionality for the ESP-IDF framework.

/// Bluetooth Low Energy advertising and scanning.
pub mod ble;
/// Physical button input handling with polling-based debounce.
pub mod button;
/// Hardware timer management and interrupt configuration.
pub mod clock;
/// RGB color representation and predefined color constants.
pub mod color;
/// GPS sensor reading via UART and NMEA parsing.
pub mod gps;
/// HTTP client for sending requests over Wi-Fi.
pub mod http;
/// Core infrastructure traits and types: [`infra::Poller`], [`infra::Switch`], and [`infra::State`].
pub mod infra;
/// `NeoPixel` LED control via the RMT peripheral.
pub mod light;
/// Inter-thread messaging with triggers, notifiers, and dispatchers.
pub mod message;
/// Thread spawning with automatic device restart on failure.
pub mod thread;
/// Time utilities for sleeping and cooperative yielding.
pub mod time;
/// Wi-Fi connection management and configuration.
pub mod wifi;
