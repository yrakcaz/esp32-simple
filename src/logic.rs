use anyhow::{anyhow, Result};
use log::info;
use std::{collections::HashSet, fmt};

#[cfg(feature = "wifi")]
use crate::http::{Client, HTTP_URL};
use crate::{
    ble::Advertiser,
    clock::Timer,
    color::{Rgb, GREEN, RED},
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
    advertiser: Advertiser,
    #[cfg(feature = "wifi")]
    http: Client<'a>,
    led: Led<'a>,
    timer: Timer<'a>,
    dispatcher: Dispatcher,
    state: State,
}

impl<'a> StateMachine<'a> {
    /// Creates a new `StateMachine` instance.
    ///
    /// # Arguments
    /// * `advertiser` - A BLE advertiser.
    /// * `http` - An HTTP client.
    /// * `led` - An LED controller.
    /// * `timer` - A timer for periodic tasks.
    /// * `dispatcher` - A dispatcher for handling triggers.
    ///
    /// # Errors
    /// Returns an error if the state machine cannot be initialized.
    pub fn new(
        advertiser: Advertiser,
        #[cfg(feature = "wifi")] http: Client<'a>,
        led: Led<'a>,
        timer: Timer<'a>,
        dispatcher: Dispatcher,
        state: State,
    ) -> Result<Self> {
        let mut led = led;
        led.set_color((&state).into())?;
        led.on()?;

        Ok(Self {
            advertiser,
            #[cfg(feature = "wifi")]
            http,
            led,
            timer,
            dispatcher,
            state,
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
                #[cfg(feature = "wifi")]
                {
                    let status = self.http.post(HTTP_URL, None)?;
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
