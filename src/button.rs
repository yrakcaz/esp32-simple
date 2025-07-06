use anyhow::{anyhow, Result};
use esp_idf_hal::gpio::{InputMode, InputPin, PinDriver};
use std::sync::{Arc, Mutex};

use crate::{
    infra::{Poller, State, Switch},
    message::{Notifier, Trigger},
    time::{sleep, yield_now},
};

/// Represents a button with a notifier and a GPIO pin.
///
/// # Type Parameters
/// * `'a` - Lifetime of the button.
/// * `T` - Type of the GPIO pin.
/// * `MODE` - Input mode of the GPIO pin.
pub struct Button<'a, T, MODE>
where
    T: InputPin,
    MODE: InputMode,
{
    notifier: Notifier,
    pin: PinDriver<'a, T, MODE>,
    state: Arc<Mutex<State>>,
}

impl<'a, T, MODE> Button<'a, T, MODE>
where
    T: InputPin,
    MODE: InputMode,
{
    /// Creates a new `Button` instance.
    ///
    /// # Arguments
    /// * `notifier` - A notifier to send button press events.
    /// * `pin` - A GPIO pin driver.
    /// * `state` - Shared state of the button.
    ///
    /// # Errors
    /// Returns an error if the button cannot be initialized.
    pub fn new(
        notifier: Notifier,
        pin: PinDriver<'a, T, MODE>,
        state: Arc<Mutex<State>>,
    ) -> Result<Self> {
        Ok(Self {
            notifier,
            pin,
            state,
        })
    }

    /// Checks if the button is pressed.
    ///
    /// # Returns
    /// `true` if the button is pressed, `false` otherwise.
    fn pressed(&self) -> bool {
        self.pin.is_low()
    }
}

impl<T, MODE> Poller for Button<'_, T, MODE>
where
    T: InputPin,
    MODE: InputMode,
{
    /// Polls the button for state changes.
    ///
    /// This function continuously checks the button state and notifies when it is pressed.
    ///
    /// # Errors
    /// Returns an error if the notifier fails or if the state cannot be toggled.
    fn poll(&mut self) -> Result<!> {
        // Using polling instead of interrupts for the button as on some boards
        // (e.g. M5Stack's Atom Lite) the interrupt pin of the button is too close
        // to the WiFi antenna which causes interference.

        loop {
            if self.pressed() {
                self.notifier.notify(Trigger::ButtonPressed)?;
                self.toggle()?;
                sleep(500);
            }
            yield_now();
        }
    }
}

impl<T, MODE> Switch for Button<'_, T, MODE>
where
    T: InputPin,
    MODE: InputMode,
{
    /// Toggles the state of the button.
    ///
    /// # Errors
    /// Returns an error if the mutex lock cannot be acquired.
    fn toggle(&mut self) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?;

        *state = match *state {
            State::On => State::Off,
            State::Off => State::On,
        };

        Ok(())
    }
}
