use anyhow::{anyhow, Result};
use esp_idf_hal::{delay::BLOCK, task::notification};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::{collections::HashSet, convert::TryFrom, num::NonZeroU32, sync::Arc};

/// Represents various triggers that can occur in the system.
///
/// # Variants
/// * `ButtonPressed` - Triggered when a button is pressed.
/// * `TimerTicked` - Triggered when a timer ticks.
/// * `DeviceFoundActive` - Triggered when an active device is found.
/// * `DeviceFoundInactive` - Triggered when an inactive device is found.
/// * `DeviceNotFound` - Triggered when no device is found.
#[derive(Debug, Eq, Hash, IntoPrimitive, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum Trigger {
    ButtonPressed = 1 << 0,
    TimerTicked = 1 << 1,
    DeviceFoundActive = 1 << 2,
    DeviceFoundInactive = 1 << 3,
    DeviceNotFound = 1 << 4,
    GpsDataAvailable = 1 << 5,
}

impl TryFrom<Trigger> for NonZeroU32 {
    /// Converts a `Trigger` into a `NonZeroU32`.
    ///
    /// # Errors
    /// Returns an error if the trigger value is invalid for `NonZeroU32`.
    type Error = anyhow::Error;

    fn try_from(trigger: Trigger) -> Result<Self, Self::Error> {
        NonZeroU32::new(trigger.into())
            .ok_or_else(|| anyhow!("Invalid value for NonZeroU32"))
    }
}

/// Represents a notifier for sending notifications.
pub struct Notifier {
    notifier: Arc<notification::Notifier>,
}

impl Notifier {
    /// Creates a new `Notifier` instance.
    ///
    /// # Arguments
    /// * `notifier` - An `Arc` of a `notification::Notifier`.
    ///
    /// # Errors
    /// Returns an error if the notifier cannot be initialized.
    pub fn new(notifier: Arc<notification::Notifier>) -> Result<Self> {
        Ok(Self { notifier })
    }

    /// Sends a notification for a given trigger.
    ///
    /// # Arguments
    /// * `trigger` - The trigger to notify.
    ///
    /// # Errors
    /// Returns an error if the notification fails.
    pub fn notify(&self, trigger: Trigger) -> Result<()> {
        unsafe {
            self.notifier.notify_and_yield(trigger.try_into()?);
        }

        Ok(())
    }
}

/// Represents a dispatcher for collecting triggers.
pub struct Dispatcher {
    notification: notification::Notification,
}

impl Dispatcher {
    /// Creates a new `Dispatcher` instance.
    ///
    /// # Errors
    /// Returns an error if the dispatcher cannot be initialized.
    pub fn new() -> Result<Self> {
        Ok(Self {
            notification: notification::Notification::new(),
        })
    }

    /// Returns a `Notifier` associated with the dispatcher.
    ///
    /// # Errors
    /// Returns an error if the notifier cannot be created.
    pub fn notifier(&self) -> Result<Notifier> {
        Notifier::new(self.notification.notifier())
    }

    /// Collects triggers from the notification system.
    ///
    /// # Returns
    /// A `HashSet` of collected triggers.
    ///
    /// # Errors
    /// Returns an error if the collection fails.
    pub fn collect(&self) -> Result<HashSet<Trigger>> {
        let mut set = HashSet::new();

        let notification = self.notification.wait(BLOCK);
        if let Some(notification) = notification {
            let notification = notification.get();
            if notification & u32::from(Trigger::ButtonPressed) != 0 {
                set.insert(Trigger::ButtonPressed);
            }
            if notification & u32::from(Trigger::TimerTicked) != 0 {
                set.insert(Trigger::TimerTicked);
            }
            if notification & u32::from(Trigger::DeviceFoundActive) != 0 {
                set.insert(Trigger::DeviceFoundActive);
            }
            if notification & u32::from(Trigger::DeviceFoundInactive) != 0 {
                set.insert(Trigger::DeviceFoundInactive);
            }
            if notification & u32::from(Trigger::DeviceNotFound) != 0 {
                set.insert(Trigger::DeviceNotFound);
            }
            if notification & u32::from(Trigger::GpsDataAvailable) != 0 {
                set.insert(Trigger::GpsDataAvailable);
            }
        }

        Ok(set)
    }
}
