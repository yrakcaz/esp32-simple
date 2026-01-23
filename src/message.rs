use anyhow::{anyhow, Result};
use esp_idf_hal::{delay::BLOCK, task::notification};
use std::{
    collections::HashSet, fmt::Debug, hash::Hash, num::NonZeroU32, sync::Arc,
};

pub trait Trigger: Debug + Eq + Hash + Sized + Send + Sync + 'static {
    const ALL: &[Self];

    fn as_u32(&self) -> u32;
}

#[macro_export]
macro_rules! trigger_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident = $value:expr),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr(u32)]
        $vis enum $name {
            $($variant = $value),*
        }

        impl $crate::message::Trigger for $name {
            const ALL: &[Self] = &[
                $(Self::$variant),*
            ];

            fn as_u32(&self) -> u32 {
                match self {
                    $(Self::$variant => $value),*
                }
            }
        }
    };
}

fn trigger_to_nonzero<T: Trigger>(trigger: &T) -> Result<NonZeroU32> {
    NonZeroU32::new(trigger.as_u32())
        .ok_or_else(|| anyhow!("Invalid value for NonZeroU32"))
}

/// Represents a notifier for sending notifications.
///
/// # Type Parameters
/// * `T` - The trigger type implementing the `Trigger` trait.
pub struct Notifier<T: Trigger> {
    notifier: Arc<notification::Notifier>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Trigger> Notifier<T> {
    /// Creates a new `Notifier` instance.
    ///
    /// # Arguments
    /// * `notifier` - An `Arc` of a `notification::Notifier`.
    ///
    /// # Errors
    /// Returns an error if the notifier cannot be initialized.
    pub fn new(notifier: Arc<notification::Notifier>) -> Result<Self> {
        Ok(Self {
            notifier,
            _marker: std::marker::PhantomData,
        })
    }

    /// Sends a notification for a given trigger.
    ///
    /// # Arguments
    /// * `trigger` - The trigger to notify.
    ///
    /// # Errors
    /// Returns an error if the notification fails.
    pub fn notify(&self, trigger: &T) -> Result<()> {
        unsafe {
            self.notifier.notify_and_yield(trigger_to_nonzero(trigger)?);
        }

        Ok(())
    }
}

/// Represents a dispatcher for collecting triggers.
///
/// # Type Parameters
/// * `T` - The trigger type implementing the `Trigger` trait.
pub struct Dispatcher<T: Trigger> {
    notification: notification::Notification,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Trigger> Dispatcher<T> {
    /// Creates a new `Dispatcher` instance.
    ///
    /// # Errors
    /// Returns an error if the dispatcher cannot be initialized.
    pub fn new() -> Result<Self> {
        Ok(Self {
            notification: notification::Notification::new(),
            _marker: std::marker::PhantomData,
        })
    }

    /// Returns a `Notifier` associated with the dispatcher.
    ///
    /// # Errors
    /// Returns an error if the notifier cannot be created.
    pub fn notifier(&self) -> Result<Notifier<T>> {
        Notifier::new(self.notification.notifier())
    }

    /// Collects triggers from the notification system.
    ///
    /// # Returns
    /// A `HashSet` of collected triggers.
    ///
    /// # Errors
    /// Returns an error if the collection fails.
    pub fn collect(&self) -> Result<HashSet<&'static T>> {
        let mut set = HashSet::new();

        let notification = self.notification.wait(BLOCK);
        if let Some(notification) = notification {
            let bits = notification.get();
            for trigger in T::ALL {
                if bits & trigger.as_u32() != 0 {
                    set.insert(trigger);
                }
            }
        }

        Ok(set)
    }
}
