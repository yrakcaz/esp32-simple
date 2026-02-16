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
/// * `TR` - The trigger type implementing the `Trigger` trait.
pub struct Button<'a, T, MODE, TR>
where
    T: InputPin,
    MODE: InputMode,
    TR: Trigger,
{
    notifier: Notifier<TR>,
    trigger: &'static TR,
    pin: PinDriver<'a, T, MODE>,
    state: Arc<Mutex<State>>,
}

impl<'a, T, MODE, TR> Button<'a, T, MODE, TR>
where
    T: InputPin,
    MODE: InputMode,
    TR: Trigger,
{
    /// Creates a new `Button` instance.
    ///
    /// # Arguments
    /// * `notifier` - A notifier to send button press events.
    /// * `trigger` - The trigger to emit when the button is pressed.
    /// * `pin` - A GPIO pin driver.
    /// * `state` - Shared state of the button.
    ///
    /// # Returns
    /// A new `Button` instance.
    ///
    /// # Errors
    /// Returns an error if the button cannot be initialized.
    pub fn new(
        notifier: Notifier<TR>,
        trigger: &'static TR,
        pin: PinDriver<'a, T, MODE>,
        state: Arc<Mutex<State>>,
    ) -> Result<Self> {
        Ok(Self {
            notifier,
            trigger,
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

impl<T, MODE, TR> Poller for Button<'_, T, MODE, TR>
where
    T: InputPin,
    MODE: InputMode,
    TR: Trigger,
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
                self.notifier.notify(self.trigger)?;
                self.toggle()?;
                sleep(500);
            }
            yield_now();
        }
    }
}

impl<T, MODE, TR> Switch for Button<'_, T, MODE, TR>
where
    T: InputPin,
    MODE: InputMode,
    TR: Trigger,
{
    /// Toggles the state of the button.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the mutex lock cannot be acquired.
    fn toggle(&mut self) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?;

        state.toggle();

        Ok(())
    }
}
