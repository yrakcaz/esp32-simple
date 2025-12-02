#![feature(never_type)]

/// The main library module for the project.
///
/// This module re-exports all submodules, providing a central entry point for the library.
///
/// # Modules
/// * `ble` - Bluetooth Low Energy (BLE) functionality.
/// * `button` - Button handling and state management.
/// * `clock` - Timer and clock-related functionality.
/// * `color` - RGB color utilities.
/// * `gps` - GPS functionality.
/// * `http` - HTTP client and server functionality.
/// * `infra` - Infrastructure traits and utilities.
/// * `light` - LED light control.
/// * `logic` - Application logic and state machine.
/// * `message` - Messaging and notification system.
/// * `thread` - Threading utilities.
/// * `time` - Time-related utilities.
/// * `wifi` - Wi-Fi connectivity and management.
pub mod ble;
pub mod button;
pub mod clock;
pub mod color;
pub mod gps;
pub mod http;
pub mod infra;
pub mod light;
pub mod message;
pub mod thread;
pub mod time;
pub mod wifi;
