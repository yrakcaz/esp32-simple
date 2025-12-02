use anyhow::{anyhow, Result};
use esp32_nimble::{
    enums::{PowerLevel, PowerType},
    BLEAdvertisementData, BLEDevice, BLEScan,
};
use esp_idf_hal::task::block_on;
use std::sync::{Arc, Mutex};

use crate::{
    clock::Timer,
    infra::{Poller, State, Switch},
    message::{Notifier, Trigger},
};

const POWER_LEVEL: PowerLevel = PowerLevel::N0; // 0 dBm
const SCAN_FREQ: u64 = 1;

/// Initializes the BLE device with the specified power levels for advertising and scanning.
///
/// # Errors
/// Returns an error if the BLE device cannot be configured with the specified power levels.
pub fn initialize_default() -> Result<()> {
    let device = BLEDevice::take();
    device.set_power(PowerType::Advertising, POWER_LEVEL)?;
    device.set_power(PowerType::Scan, POWER_LEVEL)?;

    Ok(())
}

/// Represents a BLE advertiser.
///
/// # Type Parameters
/// * `'a` - Lifetime of the advertiser.
pub struct Advertiser {
    name: String,
    state: State,
}

impl Advertiser {
    /// Creates a new `Advertiser` instance.
    ///
    /// # Arguments
    /// * `name` - Application name to use in BLE advertisements.
    /// * `state` - Initial state of the advertiser.
    ///
    /// # Errors
    /// Returns an error if the advertiser cannot be initialized.
    pub fn new(name: &str, state: State) -> Result<Self> {
        let ret = Self {
            name: name.to_string(),
            state,
        };
        ret.apply()?;

        Ok(ret)
    }

    /// Applies the current state to the BLE advertiser.
    ///
    /// # Errors
    /// Returns an error if the BLE device or advertising data cannot be configured.
    fn apply(&self) -> Result<()> {
        let device = BLEDevice::take();
        let advertising = device.get_advertising();
        let name = match self.state {
            // TODO: This doesn't take into account the fact that multiple devices could be nearby.
            //       That could be handled with some kind of an ID mechanism...
            State::On => format!("{}-Active", self.name),
            State::Off => format!("{}-Inactive", self.name),
        };

        advertising
            .lock()
            .set_data(BLEAdvertisementData::new().name(&name))?;
        advertising.lock().start()?;

        Ok(())
    }
}

impl Switch for Advertiser {
    /// Toggles the state of the advertiser.
    ///
    /// # Errors
    /// Returns an error if the state cannot be toggled or applied.
    fn toggle(&mut self) -> Result<()> {
        self.state = match self.state {
            State::On => State::Off,
            State::Off => State::On,
        };

        self.apply()
    }
}

/// Represents a BLE scanner.
///
/// # Type Parameters
/// * `'a` - Lifetime of the scanner.
pub struct Scanner<'a> {
    notifier: Notifier,
    timer: Timer<'a>,
    state: Arc<Mutex<State>>,
    device: &'a BLEDevice,
    scan: BLEScan,
    name: String,
}

impl<'a> Scanner<'a> {
    const WINDOW: i32 = 1000;

    /// Creates a new `Scanner` instance.
    ///
    /// # Arguments
    /// * `notifier` - A notifier to send scan results.
    /// * `timer` - A timer for scan intervals.
    /// * `state` - Shared state of the scanner.
    /// * `name` - Application name to scan for in BLE advertisements.
    ///
    /// # Errors
    /// Returns an error if the scanner cannot be initialized.
    pub fn new(
        notifier: Notifier,
        timer: Timer<'a>,
        state: Arc<Mutex<State>>,
        name: &str,
    ) -> Result<Self> {
        let device = BLEDevice::take();
        let scan = BLEScan::new();

        Ok(Self {
            notifier,
            timer,
            state,
            device,
            scan,
            name: name.to_string(),
        })
    }

    /// Performs a BLE scan.
    ///
    /// # Errors
    /// Returns an error if the scan fails.
    async fn do_scan(&mut self) -> Result<Option<Trigger>> {
        let app_name = self.name.clone();
        Ok(self
            .scan
            .start(self.device, Self::WINDOW, move |_, data| {
                data.name().and_then(|name| {
                    if name == format!("{app_name}-Active") {
                        Some(Trigger::DeviceFoundActive)
                    } else if name == format!("{app_name}-Inactive") {
                        Some(Trigger::DeviceFoundInactive)
                    } else {
                        None
                    }
                })
            })
            .await?)
    }
}

impl Poller for Scanner<'_> {
    /// Polls the BLE scanner for devices.
    ///
    /// This function continuously scans for BLE devices and notifies the results.
    ///
    /// # Errors
    /// Returns an error if the scan or notification fails.
    fn poll(&mut self) -> Result<!> {
        block_on(async {
            loop {
                self.timer.delay(SCAN_FREQ).await?;

                if let State::Off = *self
                    .state
                    .lock()
                    .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?
                {
                    continue;
                }

                let trigger = if let Some(trigger) = self.do_scan().await? {
                    trigger
                } else {
                    Trigger::DeviceNotFound
                };

                self.notifier.notify(trigger)?;
            }
        })
    }
}
