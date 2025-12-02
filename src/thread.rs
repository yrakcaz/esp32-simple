use anyhow::Result;
use esp_idf_hal::reset::restart;
use log::error;
use std::thread;

use crate::time::sleep;

/// Handles program failure by restarting the device.
///
/// This function waits for a second and then restarts the device if the program encounters an error.
pub fn failure() -> ! {
    // This program should run forever, until the device is powered off.
    // If something goes wrong and the program dies, we wait for a second and
    // then restart the device.
    sleep(1000);
    restart();
}

/// Runs the main application logic with automatic error logging and device restart on exit.
///
/// This function wraps the provided closure to ensure the device restarts
/// if the program exits. Any errors are logged with their full chain
/// before the restart occurs.
///
/// # Arguments
/// * `f` - A closure that returns a `Result`.
///
/// # Type Parameters
/// * `F` - The type of the closure.
///
/// # Returns
/// Never returns normally - either runs forever or restarts the device.
pub fn main<F>(f: F) -> !
where
    F: FnOnce() -> Result<()>,
{
    if let Err(e) = f() {
        error!("Fatal error: {:#}", e);
    }

    failure()
}

/// A guard that ensures the program restarts on thread exit.
struct ExitGuard;

impl Drop for ExitGuard {
    /// Ensures the program restarts when the thread exits.
    fn drop(&mut self) {
        failure();
    }
}

/// Spawns a new thread with a failure guard.
///
/// # Arguments
/// * `f` - A closure to execute in the new thread.
///
/// # Returns
/// A `JoinHandle` for the spawned thread.
///
/// # Type Parameters
/// * `F` - The type of the closure.
/// * `T` - The return type of the closure.
pub fn spawn<F, T>(f: F) -> thread::JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(move || {
        let _guard = ExitGuard;
        f()
    })
}
