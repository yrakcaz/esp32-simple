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

/// Initializes the BLE device with the specified power level for advertising and scanning.
///
/// # Arguments
/// * `power_level` - The power level to use for both advertising and scanning.
///
/// # Returns
/// `Ok(())` on success.
///
/// # Errors
/// Returns an error if the BLE device cannot be configured with the specified power levels.
pub fn initialize(power_level: PowerLevel) -> Result<()> {
    let device = BLEDevice::take();
    device.set_power(PowerType::Advertising, power_level)?;
    device.set_power(PowerType::Scan, power_level)?;

    Ok(())
}

/// Function type for deriving advertisement name and payload from state.
type DeriveFn = fn(&State, Option<&[u8]>) -> (String, Option<Vec<u8>>);

/// Represents a BLE advertiser.
pub struct Advertiser {
    state: State,
    payload: Option<Vec<u8>>,
    derive: DeriveFn,
}

impl Advertiser {
    /// Creates a new `Advertiser` instance.
    ///
    /// # Arguments
    /// * `state` - Initial state of the advertiser.
    /// * `derive` - Function to derive advertisement name and payload from state.
    ///
    /// # Returns
    /// A new `Advertiser` with the advertisement already applied.
    ///
    /// # Errors
    /// Returns an error if the advertisement cannot be applied.
    pub fn new(state: State, derive: DeriveFn) -> Result<Self> {
        let ret = Self {
            state,
            payload: None,
            derive,
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
        let (name, payload) = (self.derive)(&self.state, self.payload.as_deref());

        let mut data = BLEAdvertisementData::new();
        data.name(&name);
        if let Some(bytes) = &payload {
            data.manufacturer_data(bytes);
        }

        advertising.lock().set_data(&mut data)?;
        advertising.lock().start()?;

        Ok(())
    }

    /// Updates the BLE advertisement payload and re-applies the advertisement.
    ///
    /// # Arguments
    /// * `payload` - Optional new manufacturer data bytes, or `None` to clear.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the advertisement cannot be re-applied.
    pub fn set_payload(&mut self, payload: Option<Vec<u8>>) -> Result<()> {
        self.payload = payload;
        self.apply()
    }
}

impl Switch for Advertiser {
    /// Toggles the state of the advertiser.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the advertisement cannot be re-applied.
    fn toggle(&mut self) -> Result<()> {
        self.state.toggle();

        self.apply()
    }
}

/// Configuration for BLE scanning behavior.
///
/// # Type Parameters
/// * `T` - The trigger type implementing the `Trigger` trait.
pub struct ScannerConfig<T: Trigger> {
    triggers: fn(&str) -> Option<&'static T>,
    default_trigger: &'static T,
    payload_trigger: &'static T,
    scan_freq_hz: u64,
}

impl<T: Trigger> ScannerConfig<T> {
    /// Creates a new scan configuration.
    ///
    /// # Arguments
    /// * `triggers` - Function to look up a trigger by BLE device name.
    /// * `default_trigger` - Trigger to emit when no matching device is found.
    /// * `payload_trigger` - Store payload when this trigger matches.
    /// * `scan_freq_hz` - Scan frequency in Hz.
    ///
    /// # Returns
    /// A new `ScannerConfig` instance.
    #[must_use]
    pub fn new(
        triggers: fn(&str) -> Option<&'static T>,
        default_trigger: &'static T,
        payload_trigger: &'static T,
        scan_freq_hz: u64,
    ) -> Self {
        Self {
            triggers,
            default_trigger,
            payload_trigger,
            scan_freq_hz,
        }
    }
}

/// Represents a BLE scanner.
///
/// # Type Parameters
/// * `'a` - Lifetime of the scanner.
/// * `T` - The trigger type implementing the `Trigger` trait.
pub struct Scanner<'a, T: Trigger> {
    notifier: Notifier<T>,
    timer: Timer<'a, T>,
    state: Arc<Mutex<State>>,
    payload: Arc<Mutex<Option<Vec<u8>>>>,
    device: &'a BLEDevice,
    scan: BLEScan,
    config: ScannerConfig<T>,
}

impl<'a, T: Trigger> Scanner<'a, T> {
    /// BLE scan window duration in milliseconds.
    const WINDOW: i32 = 1000;

    /// Creates a new `Scanner` instance.
    ///
    /// # Arguments
    /// * `notifier` - A notifier to send scan results.
    /// * `timer` - A timer for scan intervals.
    /// * `state` - Shared state of the scanner.
    /// * `payload` - Shared storage for BLE payload data.
    /// * `config` - Scan configuration (triggers, frequency, etc.).
    ///
    /// # Returns
    /// A new `Scanner` ready to poll.
    ///
    /// # Errors
    /// Returns an error if the scanner cannot be initialized.
    pub fn new(
        notifier: Notifier<T>,
        timer: Timer<'a, T>,
        state: Arc<Mutex<State>>,
        payload: Arc<Mutex<Option<Vec<u8>>>>,
        config: ScannerConfig<T>,
    ) -> Result<Self> {
        let device = BLEDevice::take();
        let scan = BLEScan::new();

        Ok(Self {
            notifier,
            timer,
            state,
            payload,
            device,
            scan,
            config,
        })
    }

    /// Performs a BLE scan.
    ///
    /// # Errors
    /// Returns an error if the scan fails.
    async fn do_scan(&mut self) -> Result<Option<&'static T>> {
        let triggers = self.config.triggers;
        let payload = Arc::clone(&self.payload);
        let payload_trigger = self.config.payload_trigger;
        Ok(self
            .scan
            .start(self.device, Self::WINDOW, move |_, data| {
                data.name().and_then(|name| {
                    let name = String::from_utf8_lossy(name);
                    if let Some(trigger) = triggers(&name) {
                        if trigger == payload_trigger {
                            if let Some(mfg) = data.manufacture_data() {
                                if let Ok(mut stored) = payload.lock() {
                                    // manufacture_data() splits the raw bytes into a
                                    // 2-byte company_identifier and the remaining payload.
                                    // We reconstruct the original bytes here.
                                    let mut full = mfg
                                        .company_identifier
                                        .to_le_bytes()
                                        .to_vec();
                                    full.extend_from_slice(mfg.payload);
                                    *stored = Some(full);
                                }
                            }
                        }
                        Some(trigger)
                    } else {
                        None
                    }
                })
            })
            .await?)
    }
}

impl<T: Trigger> Poller for Scanner<'_, T> {
    /// Polls the BLE scanner for devices.
    ///
    /// This function continuously scans for BLE devices and notifies the results.
    ///
    /// # Errors
    /// Returns an error if the scan or notification fails.
    fn poll(&mut self) -> Result<!> {
        block_on(async {
            loop {
                self.timer.delay(self.config.scan_freq_hz).await?;

                if self
                    .state
                    .lock()
                    .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?
                    .is_off()
                {
                    continue;
                }

                let trigger = if let Some(trigger) = self.do_scan().await? {
                    trigger
                } else {
                    self.config.default_trigger
                };

                self.notifier.notify(trigger)?;
            }
        })
    }
}
