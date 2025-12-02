use anyhow::{anyhow, Result};
use esp_idf_hal::uart::UartRxDriver;
use nmea::{Nmea, SentenceType};
use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

use crate::{
    infra::{Poller, State},
    message::Notifier,
    time::yield_now,
};

const READ_TIMEOUT: u32 = 1000;

pub struct Reading {
    latitude: f64,
    longitude: f64,
    altitude: f32,
}

impl Reading {
    fn new(latitude: f64, longitude: f64, altitude: f32) -> Self {
        Self {
            latitude,
            longitude,
            altitude,
        }
    }
}

impl Display for Reading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Lat: {}, Lon: {}, Alt: {}",
            self.latitude, self.longitude, self.altitude
        )
    }
}

pub struct Sensor<'a> {
    notifier: Notifier,
    state: Arc<Mutex<State>>,
    uart: UartRxDriver<'a>,
    data: Arc<Mutex<Option<Reading>>>,
    buffer: String,
}

impl<'a> Sensor<'a> {
    pub fn new(
        notifier: Notifier,
        state: Arc<Mutex<State>>,
        uart: UartRxDriver<'a>,
        data: Arc<Mutex<Option<Reading>>>,
    ) -> Self {
        Self {
            notifier,
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

                {
                    let complete = &self.buffer[..range_end];
                    for line in complete.split("\r\n") {
                        if line.trim().is_empty() {
                            continue;
                        }

                        let mut parser = Nmea::default();
                        if let Ok(SentenceType::GGA) = parser.parse(line) {
                            if let (Some(lat), Some(lon), Some(alt)) = (
                                parser.latitude(),
                                parser.longitude(),
                                parser.altitude(),
                            ) {
                                ret = Some(Reading::new(lat, lon, alt));
                            }
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

impl Poller for Sensor<'_> {
    fn poll(&mut self) -> Result<!> {
        loop {
            yield_now();

            if let State::Off = *self
                .state
                .lock()
                .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?
            {
                continue;
            }

            if let Some(reading) = self.read()? {
                let mut data = self
                    .data
                    .lock()
                    .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?;

                *data = Some(reading);
                self.notifier
                    .notify(crate::message::Trigger::GpsDataAvailable)?;
            }
        }
    }
}
