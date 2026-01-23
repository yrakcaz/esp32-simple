use anyhow::Result;

/// A trait representing a poller that performs periodic tasks.
///
/// # Errors
/// This trait's `poll` method returns an error if the polling operation fails.
pub trait Poller {
    /// Polls for periodic tasks.
    ///
    /// # Errors
    /// Returns an error if the polling operation fails.
    fn poll(&mut self) -> Result<!>;
}

/// Represents an on/off state, optionally carrying additional data when on.
///
/// # Type Parameters
/// * `T` - The type of additional data carried when in the `On` state. Defaults to `()`.
///
/// # Variants
/// * `Off` - The switch is turned off.
/// * `On(Option<T>)` - The switch is on, optionally with additional data.
pub enum State<T = ()> {
    Off,
    On(Option<T>),
}

impl<T> State<T> {
    /// Creates a new `State` in the `On` position with no additional data.
    pub const fn on() -> Self {
        State::On(None)
    }

    /// Creates a new `State` in the `Off` position.
    #[must_use]
    pub const fn off() -> Self {
        State::Off
    }

    /// Returns `true` if the state is `Off`.
    pub fn is_off(&self) -> bool {
        matches!(self, State::Off)
    }

    /// Returns `true` if the state is `On`.
    pub fn is_on(&self) -> bool {
        matches!(self, State::On(_))
    }

    /// Toggles between `On` and `Off`, clearing any additional data.
    pub fn toggle(&mut self) {
        *self = match self {
            State::On(_) => State::Off,
            State::Off => State::On(None),
        };
    }
}

/// A trait representing a switch that can toggle its state.
///
/// # Errors
/// This trait's `toggle` method returns an error if the toggle operation fails.
pub trait Switch {
    /// Toggles the state of the switch.
    ///
    /// # Errors
    /// Returns an error if the toggle operation fails.
    fn toggle(&mut self) -> Result<()>;
}
