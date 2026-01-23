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

pub struct Reading {
    latitude: f64,
    longitude: f64,
    speed_mps: Option<f32>,
}

impl Reading {
    #[must_use]
    pub fn new(latitude: f64, longitude: f64, speed_mps: Option<f32>) -> Self {
        Self {
            latitude,
            longitude,
            speed_mps,
        }
    }

    #[must_use]
    pub fn latitude(&self) -> f64 {
        self.latitude
    }

    #[must_use]
    pub fn longitude(&self) -> f64 {
        self.longitude
    }

    #[must_use]
    pub fn speed_mps(&self) -> Option<f32> {
        self.speed_mps
    }
}

impl Display for Reading {
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
