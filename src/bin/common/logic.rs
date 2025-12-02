use anyhow::{anyhow, Result};
use log::info;
use std::{
    collections::HashSet,
    fmt,
    sync::{Arc, Mutex},
};

use esp_layground::{
    ble::Advertiser,
    clock::Timer,
    color::{Rgb, GREEN, RED},
    gps::Reading,
    http::Client,
    infra::{self, Switch},
    light::Led,
    message::{Dispatcher, Trigger},
};

macro_rules! func {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);

        match &name[..name.len() - 3].rfind(':') {
            Some(pos) => &name[pos + 1..name.len() - 3],
            None => &name[..name.len() - 3],
        }
    }};
}

/// Represents the state of the application.
///
/// # Variants
/// * `On` - The application is active.
/// * `Off` - The application is inactive.
/// * `ActiveDeviceNearby` - An active device is detected nearby.
/// * `InactiveDeviceNearby` - An inactive device is detected nearby.
#[derive(PartialEq)]
pub enum State {
    On,
    Off,
    ActiveDeviceNearby,
    InactiveDeviceNearby,
}

impl fmt::Display for State {
    /// Formats the state as a string.
    ///
    /// # Returns
    /// A string representation of the state.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::On => write!(f, "On"),
            State::Off => write!(f, "Off"),
            State::ActiveDeviceNearby => write!(f, "ActiveDeviceNearby"),
            State::InactiveDeviceNearby => write!(f, "InactiveDeviceNearby"),
        }
    }
}

impl From<infra::State> for State {
    fn from(state: infra::State) -> Self {
        match state {
            infra::State::On => State::On,
            infra::State::Off => State::Off,
        }
    }
}

impl From<&State> for Rgb {
    /// Converts a `State` to an `Rgb` color.
    ///
    /// # Returns
    /// An `Rgb` color corresponding to the state.
    fn from(state: &State) -> Self {
        match state {
            State::On | State::ActiveDeviceNearby => GREEN,
            State::Off | State::InactiveDeviceNearby => RED,
        }
    }
}

/// Represents the state machine for the application.
///
/// # Type Parameters
/// * `'a` - Lifetime of the state machine.
pub struct StateMachine<'a> {
    state: State,
    dispatcher: Dispatcher,
    advertiser: Advertiser,
    led: Led<'a>,
    timer: Timer<'a>,
    location: Option<Arc<Mutex<Option<Reading>>>>,
    http: Option<Client<'a>>,
    url: Option<&'a str>,
}

impl<'a> StateMachine<'a> {
    pub fn new(
        state: State,
        dispatcher: Dispatcher,
        advertiser: Advertiser,
        led: Led<'a>,
        timer: Timer<'a>,
        location: Option<Arc<Mutex<Option<Reading>>>>,
        http: Option<Client<'a>>,
    ) -> Result<Self> {
        let mut led = led;
        led.set_color((&state).into())?;
        led.on()?;

        let url =
            if http.is_some() {
                Some(option_env!("HTTP_URL").ok_or_else(|| {
                    anyhow!("HTTP_URL environment variable not set")
                })?)
            } else {
                None
            };

        Ok(Self {
            state,
            dispatcher,
            advertiser,
            led,
            timer,
            location,
            http,
            url,
        })
    }

    /// Handles the button pressed trigger.
    ///
    /// # Errors
    /// Returns an error if the advertiser state cannot be toggled.
    fn handle_button_pressed(&mut self) -> Result<()> {
        info!("{}", func!());

        self.state = match self.state {
            State::Off => State::On,
            _ => State::Off,
        };

        self.advertiser.toggle()
    }

    /// Handles the device found active trigger.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP POST request fails.
    #[allow(clippy::unnecessary_wraps)]
    fn handle_device_found_active(&mut self) -> Result<()> {
        info!("{}", func!());

        self.state = match self.state {
            State::Off => State::Off,
            State::ActiveDeviceNearby => State::ActiveDeviceNearby,
            _ => {
                if let (Some(http), Some(url)) = (&mut self.http, self.url) {
                    let status = http.post(url, None)?;
                    info!("HTTP POST request sent, status: {}", status);
                }
                State::ActiveDeviceNearby
            }
        };

        Ok(())
    }

    /// Handles the device found inactive trigger.
    fn handle_device_found_inactive(&mut self) {
        info!("{}", func!());

        self.state = match self.state {
            State::Off => State::Off,
            _ => State::InactiveDeviceNearby,
        };
    }

    /// Handles the device not found trigger.
    fn handle_device_not_found(&mut self) {
        info!("{}", func!());

        self.state = match self.state {
            State::Off => State::Off,
            _ => State::On,
        };
    }

    /// Handles the timer ticked trigger.
    ///
    /// # Errors
    /// Returns an error if the LED state cannot be toggled.
    fn handle_timer_ticked(&mut self) -> Result<()> {
        info!("{}", func!());

        match self.state {
            State::ActiveDeviceNearby | State::InactiveDeviceNearby => {
                self.led.toggle()
            } // Blinking
            _ => Ok(()),
        }
    }

    // FIXME don't forget to add missing doc everywhere... fmt+clippy! and update TODO and README..
    fn handle_gps_data(&mut self) -> Result<()> {
        info!("{}", func!());

        if let Some(location) = &self.location {
            let data = location
                .lock()
                .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?;
            if let Some(reading) = data.as_ref() {
                // FIXME What we actually need to do is feed this into something that will compute and keep track of the average and max speeds.
                //       These data then need to be transmitted through BLE to the server then through HTTP from the server.
                info!("GPS Reading: {}", reading);
            }
        }

        Ok(())
    }

    /// Handles a set of triggers.
    ///
    /// # Arguments
    /// * `triggers` - A set of triggers to handle.
    ///
    /// # Errors
    /// Returns an error if any trigger handling fails.
    fn handle_triggers(&mut self, triggers: &HashSet<Trigger>) -> Result<()> {
        info!(
            "{}: triggers: {:?}, state: {}",
            func!(),
            triggers,
            self.state
        );

        if triggers.contains(&Trigger::ButtonPressed) {
            self.handle_button_pressed()?;
        } else if triggers.contains(&Trigger::DeviceFoundActive) {
            self.handle_device_found_active()?;
        } else if triggers.contains(&Trigger::DeviceFoundInactive) {
            self.handle_device_found_inactive();
        } else if triggers.contains(&Trigger::DeviceNotFound) {
            self.handle_device_not_found();
        } else if triggers.contains(&Trigger::TimerTicked) {
            self.handle_timer_ticked()?;
        } else if triggers.contains(&Trigger::GpsDataAvailable) {
            self.handle_gps_data()?;
        } else {
            Err(anyhow!("Unknown triggers: {:?}", triggers))?;
        }

        Ok(())
    }

    /// Runs the state machine.
    ///
    /// # Errors
    /// Returns an error if the state machine encounters an issue during execution.
    pub fn run(&mut self) -> Result<()> {
        loop {
            let triggers = self.dispatcher.collect()?;
            self.handle_triggers(&triggers)?;

            self.led.set_color((&self.state).into())?;
            if self.state == State::On || self.state == State::Off {
                self.timer.off()?;
                self.led.on()?;
            } else {
                self.timer.on()?;
            }
        }
    }
}
