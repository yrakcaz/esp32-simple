use anyhow::Result;
use esp_idf_hal::rmt::{FixedLengthSignal, PinState, Pulse, TxRmtDriver};
use std::time::Duration;

use crate::{
    color::{Rgb, BLACK},
    infra::{State, Switch},
};

/// Sends an RGB color value to a `NeoPixel` LED using the RMT peripheral.
///
/// # Arguments
///
/// * `rgb` - An `Rgb` struct containing the red, green, and blue color values.
/// * `tx` - A mutable reference to a `TxRmtDriver` used to transmit the signal.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the operation was successful, or an `anyhow::Error` if an error occurred.
///
/// # Errors
///
/// This function will return an error if:
///
/// * There is an issue with the RMT driver, such as failing to retrieve the counter clock frequency.
/// * There is an issue creating the pulses with the specified durations.
/// * There is an issue setting the signal pulses.
/// * There is an issue starting the transmission.
fn neopixel(rgb: &Rgb, tx: &mut TxRmtDriver) -> Result<()> {
    let color: u32 = rgb.into();
    let ticks_hz = tx.counter_clock()?;
    let (t0_high, t0_low, t1_high, t1_low) = (
        Pulse::new_with_duration(
            ticks_hz,
            PinState::High,
            &Duration::from_nanos(350),
        )?,
        Pulse::new_with_duration(
            ticks_hz,
            PinState::Low,
            &Duration::from_nanos(800),
        )?,
        Pulse::new_with_duration(
            ticks_hz,
            PinState::High,
            &Duration::from_nanos(700),
        )?,
        Pulse::new_with_duration(
            ticks_hz,
            PinState::Low,
            &Duration::from_nanos(600),
        )?,
    );
    let mut signal = FixedLengthSignal::<24>::new();
    for i in (0..24).rev() {
        let p = 2_u32.pow(i);
        let bit: bool = p & color != 0;
        let (high_pulse, low_pulse) = if bit {
            (t1_high, t1_low)
        } else {
            (t0_high, t0_low)
        };
        signal.set(23 - i as usize, &(high_pulse, low_pulse))?;
    }
    tx.start_blocking(&signal)?;
    Ok(())
}

/// Represents an LED with color and state control.
///
/// # Type Parameters
/// * `'a` - Lifetime of the LED.
pub struct Led<'a> {
    color: Rgb,
    state: State,
    tx_rmt: TxRmtDriver<'a>,
}

impl<'a> Led<'a> {
    /// Creates a new `Led` instance.
    ///
    /// # Arguments
    /// * `tx_rmt` - A `TxRmtDriver` for controlling the LED.
    ///
    /// # Errors
    /// Returns an error if the LED cannot be initialized.
    pub fn new(tx_rmt: TxRmtDriver<'a>) -> Result<Self> {
        let mut ret = Self {
            tx_rmt,
            color: BLACK,
            state: State::off(),
        };
        ret.apply()?;

        Ok(ret)
    }

    /// Applies the current state and color to the LED.
    ///
    /// # Errors
    /// Returns an error if the LED state or color cannot be applied.
    fn apply(&mut self) -> Result<()> {
        match self.state {
            State::On(_) => neopixel(&self.color, &mut self.tx_rmt),
            State::Off => neopixel(&BLACK, &mut self.tx_rmt),
        }
    }

    /// Sets the color of the LED.
    ///
    /// # Arguments
    /// * `color` - The new color for the LED.
    ///
    /// # Errors
    /// Returns an error if the color cannot be applied.
    pub fn set_color(&mut self, color: Rgb) -> Result<()> {
        self.color = color;

        self.apply()
    }

    /// Turns on the LED.
    ///
    /// # Errors
    /// Returns an error if the LED cannot be turned on.
    pub fn on(&mut self) -> Result<()> {
        self.state = State::on();

        self.apply()
    }

    /// Turns off the LED.
    ///
    /// # Errors
    /// Returns an error if the LED cannot be turned off.
    pub fn off(&mut self) -> Result<()> {
        self.state = State::off();

        self.apply()
    }
}

impl Switch for Led<'_> {
    /// Toggles the state of the LED.
    ///
    /// # Errors
    /// Returns an error if the LED state cannot be toggled.
    fn toggle(&mut self) -> Result<()> {
        match self.state {
            State::On(_) => self.off(),
            State::Off => self.on(),
        }
    }
}
