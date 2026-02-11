use anyhow::{anyhow, Result};
use esp_idf_hal::uart::UartRxDriver;
use nmea::{Nmea, SentenceType};
use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

use crate::{
    infra::{Poller, State},
    message::{Notifier, Trigger},
    time::yield_now,
};

const READ_TIMEOUT: u32 = 1000;

/// A GPS reading containing position and optional speed data.
///
/// # Fields
/// * `latitude` - Latitude in decimal degrees.
/// * `longitude` - Longitude in decimal degrees.
/// * `speed_mps` - Speed in meters per second, if available from the GPS fix.
pub struct Reading {
    latitude: f64,
    longitude: f64,
    speed_mps: Option<f32>,
}

impl Reading {
    /// Creates a new `Reading` with the given position and optional speed.
    ///
    /// # Arguments
    /// * `latitude` - Latitude in decimal degrees.
    /// * `longitude` - Longitude in decimal degrees.
    /// * `speed_mps` - Speed in meters per second, or `None` if unavailable.
    ///
    /// # Returns
    /// A new `Reading` instance.
    #[must_use]
    pub fn new(latitude: f64, longitude: f64, speed_mps: Option<f32>) -> Self {
        Self {
            latitude,
            longitude,
            speed_mps,
        }
    }

    /// Returns the latitude in decimal degrees.
    ///
    /// # Returns
    /// The latitude as `f64`.
    #[must_use]
    pub fn latitude(&self) -> f64 {
        self.latitude
    }

    /// Returns the longitude in decimal degrees.
    ///
    /// # Returns
    /// The longitude as `f64`.
    #[must_use]
    pub fn longitude(&self) -> f64 {
        self.longitude
    }

    /// Returns the speed in meters per second, if available.
    ///
    /// # Returns
    /// `Some(speed)` if the GPS fix includes speed data, `None` otherwise.
    #[must_use]
    pub fn speed_mps(&self) -> Option<f32> {
        self.speed_mps
    }
}

impl Display for Reading {
    /// Formats the reading as `Lat: {lat}, Lon: {lon}, Speed: {speed} m/s` (or `N/A` if no speed).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Lat: {}, Lon: {}, Speed: ",
            self.latitude, self.longitude
        )?;
        match self.speed_mps {
            Some(s) => write!(f, "{s:.2} m/s"),
            None => write!(f, "N/A"),
        }
    }
}

/// Represents a GPS sensor.
///
/// # Type Parameters
/// * `'a` - Lifetime of the sensor.
/// * `T` - The trigger type implementing the `Trigger` trait.
pub struct Sensor<'a, T: Trigger> {
    notifier: Notifier<T>,
    trigger: &'static T,
    state: Arc<Mutex<State>>,
    uart: UartRxDriver<'a>,
    data: Arc<Mutex<Option<Reading>>>,
    buffer: String,
}

impl<'a, T: Trigger> Sensor<'a, T> {
    /// Creates a new GPS `Sensor`.
    ///
    /// # Arguments
    /// * `notifier` - A notifier to send GPS data available events.
    /// * `trigger` - The trigger to emit when a new reading is available.
    /// * `state` - Shared on/off state controlling whether the sensor reads data.
    /// * `uart` - UART receive driver connected to the GPS module.
    /// * `data` - Shared storage for the latest GPS reading.
    ///
    /// # Returns
    /// A new `Sensor` instance ready to poll.
    pub fn new(
        notifier: Notifier<T>,
        trigger: &'static T,
        state: Arc<Mutex<State>>,
        uart: UartRxDriver<'a>,
        data: Arc<Mutex<Option<Reading>>>,
    ) -> Self {
        Self {
            notifier,
            trigger,
            state,
            uart,
            data,
            buffer: String::new(),
        }
    }

    fn read(&mut self) -> Result<Option<Reading>> {
        let mut ret = None;
        let mut buf = [0u8; 256];

        let n = self.uart.read(&mut buf, READ_TIMEOUT)?;
        if n > 0 {
            let s = String::from_utf8_lossy(&buf[..n]);
            self.buffer.push_str(&s);

            if let Some(last_idx) = self.buffer.rfind("\r\n") {
                let range_end = last_idx + 2;

                let complete = &self.buffer[..range_end];
                for line in complete.split("\r\n") {
                    if line.trim().is_empty() {
                        continue;
                    }

                    let mut parser = Nmea::default();
                    if let Ok(SentenceType::RMC) = parser.parse(line) {
                        if let (Some(lat), Some(lon)) =
                            (parser.latitude(), parser.longitude())
                        {
                            let speed_mps = parser
                                .speed_over_ground
                                .map(|knots| knots * 0.514_444);
                            ret = Some(Reading::new(lat, lon, speed_mps));
                        }
                    }
                }

                self.buffer.drain(..range_end);
            }

            if self.buffer.len() > 4096 {
                self.buffer.clear();
            }
        }

        Ok(ret)
    }
}

impl<T: Trigger> Poller for Sensor<'_, T> {
    /// Continuously reads NMEA sentences from the UART and publishes GPS readings.
    ///
    /// Skips reading when the shared state is off. When a valid RMC sentence is parsed,
    /// stores the reading in the shared data mutex and sends a notification.
    ///
    /// # Errors
    /// Returns an error if UART reading, mutex locking, or notification fails.
    fn poll(&mut self) -> Result<!> {
        loop {
            yield_now();

            if self
                .state
                .lock()
                .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?
                .is_off()
            {
                continue;
            }

            if let Some(reading) = self.read()? {
                let mut data = self
                    .data
                    .lock()
                    .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?;

                *data = Some(reading);
                self.notifier.notify(self.trigger)?;
            }
        }
    }
}
