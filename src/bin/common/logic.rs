use anyhow::Result;
use std::collections::HashSet;

use esp_layground::{
    ble::Advertiser,
    clock::Timer,
    color::{Rgb, GREEN, RED},
    infra::{self, Switch},
    light::Led,
    message::Dispatcher,
    trigger_enum,
};

// FIXME don't forget to add missing doc everywhere... and update README. And update instructions.md so it's automatic in the future..

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
pub(crate) use func;

macro_rules! trace_func {
    () => {
        log::debug!("{}", $crate::common::logic::func!())
    };
}
pub(crate) use trace_func;

trigger_enum! {
    #[derive(Debug, Eq, Hash, PartialEq)]
    pub enum Trigger {
        ButtonPressed = 1 << 0,
        TimerTicked = 1 << 1,
        DeviceFoundActive = 1 << 2,
        DeviceFoundInactive = 1 << 3,
        DeviceNotFound = 1 << 4,
        GpsDataAvailable = 1 << 5,
    }
}

/// Represents whether a nearby device is active or inactive.
#[derive(PartialEq)]
pub enum DeviceNearby {
    Active,
    Inactive,
}

/// Application state: either Off or On with optional device nearby info.
pub type State = infra::State<DeviceNearby>;

/// Extension trait for app-specific State behavior.
pub trait StateExt {
    fn to_str(&self) -> &'static str;
    fn to_color(&self) -> Rgb;
}

impl StateExt for State {
    fn to_str(&self) -> &'static str {
        match self {
            State::Off => "Off",
            State::On(None) => "On",
            State::On(Some(DeviceNearby::Active)) => "ActiveDeviceNearby",
            State::On(Some(DeviceNearby::Inactive)) => "InactiveDeviceNearby",
        }
    }

    fn to_color(&self) -> Rgb {
        match self {
            State::On(None) | State::On(Some(DeviceNearby::Active)) => GREEN,
            State::Off | State::On(Some(DeviceNearby::Inactive)) => RED,
        }
    }
}

pub struct Core<'a> {
    pub state: State,
    pub dispatcher: Dispatcher<Trigger>,
    pub advertiser: Advertiser,
    pub led: Led<'a>,
    pub timer: Timer<'a, Trigger>,
}

impl<'a> Core<'a> {
    /// Creates a new core with initialized LED.
    ///
    /// # Errors
    /// Returns an error if LED initialization fails.
    pub fn new(
        state: State,
        dispatcher: Dispatcher<Trigger>,
        advertiser: Advertiser,
        mut led: Led<'a>,
        timer: Timer<'a, Trigger>,
    ) -> Result<Self> {
        led.set_color(state.to_color())?;
        led.on()?;

        Ok(Self {
            state,
            dispatcher,
            advertiser,
            led,
            timer,
        })
    }

    /// Handles the timer ticked trigger (LED blinking when device nearby).
    ///
    /// # Errors
    /// Returns an error if the LED state cannot be toggled.
    pub fn handle_timer_ticked(&mut self) -> Result<()> {
        trace_func!();

        match self.state {
            State::On(Some(_)) => self.led.toggle(),
            _ => Ok(()),
        }
    }

    /// Handles the device found inactive trigger.
    pub fn handle_device_found_inactive(&mut self) {
        trace_func!();

        if self.state.is_on() {
            self.state = State::On(Some(DeviceNearby::Inactive));
        }
    }

    /// Handles the device not found trigger.
    pub fn handle_device_not_found(&mut self) {
        trace_func!();

        if self.state.is_on() {
            self.state = State::on();
        }
    }

    /// Handles common triggers, returning true if handled.
    ///
    /// Handles: ButtonPressed, DeviceFoundActive, DeviceFoundInactive,
    /// DeviceNotFound, TimerTicked.
    ///
    /// # Errors
    /// Returns an error if trigger handling fails.
    pub fn handle_common_triggers(
        &mut self,
        triggers: &HashSet<&'static Trigger>,
        on_button_pressed: impl FnOnce(&mut Self) -> Result<()>,
        on_device_found_active: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<bool> {
        log::debug!(
            "{}: triggers: {:?}, state: {}",
            func!(),
            triggers,
            self.state.to_str()
        );

        let mut handled = true;
        if triggers.contains(&Trigger::ButtonPressed) {
            on_button_pressed(self)?;
        } else if triggers.contains(&Trigger::DeviceFoundActive) {
            on_device_found_active(self)?;
        } else if triggers.contains(&Trigger::DeviceFoundInactive) {
            self.handle_device_found_inactive();
        } else if triggers.contains(&Trigger::DeviceNotFound) {
            self.handle_device_not_found();
        } else if triggers.contains(&Trigger::TimerTicked) {
            self.handle_timer_ticked()?;
        } else {
            handled = false;
        }

        Ok(handled)
    }

    /// Updates LED state based on current state.
    ///
    /// # Errors
    /// Returns an error if LED or timer operations fail.
    pub fn update_led(&mut self) -> Result<()> {
        self.led.set_color(self.state.to_color())?;
        if matches!(self.state, State::On(None) | State::Off) {
            self.timer.off()?;
            self.led.on()?;
        } else {
            self.timer.on()?;
        }
        Ok(())
    }

    /// Runs the main loop, delegating trigger handling to the provided closure.
    ///
    /// # Errors
    /// Returns an error if any operation fails.
    pub fn run<F>(&mut self, mut handle_triggers: F) -> Result<()>
    where
        F: FnMut(&mut Self, &HashSet<&'static Trigger>) -> Result<()>,
    {
        loop {
            let triggers = self.dispatcher.collect()?;
            handle_triggers(self, &triggers)?;
            self.update_led()?;
        }
    }
}
