#[cfg(feature = "wifi")]
use anyhow::Result;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};

/// The SSID of the Wi-Fi network to connect to.
/// This value is retrieved from the environment variable `WIFI_SSID`.
const WIFI_SSID: &str = env!("WIFI_SSID");
/// The password of the Wi-Fi network to connect to.
/// This value is retrieved from the environment variable `WIFI_PASSWORD`.
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

/// Represents a Wi-Fi connection, handling its configuration and state management.
///
/// This struct leverages the `BlockingWifi` handler from the ESP-IDF framework for managing the connection.
pub struct Connection<'a> {
    handler: BlockingWifi<EspWifi<'a>>,
}

impl<'a> Connection<'a> {
    /// Creates a new `Connection` instance with the given Wi-Fi handler and credentials.
    ///
    /// # Arguments
    ///
    /// * `handler` - The Wi-Fi handler to manage the connection.
    /// * `auth_method` - The authentication method to use (e.g., WPA2).
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be set or if the SSID/password conversion fails.
    pub fn new(
        handler: BlockingWifi<EspWifi<'a>>,
        auth_method: AuthMethod,
    ) -> Result<Self> {
        let configuration: Configuration =
            Configuration::Client(ClientConfiguration {
                auth_method,
                ssid: WIFI_SSID
                    .try_into()
                    .map_err(|()| anyhow::anyhow!("Failed to convert SSID"))?,
                password: WIFI_PASSWORD
                    .try_into()
                    .map_err(|()| anyhow::anyhow!("Failed to convert password"))?,
                ..Default::default()
            });

        let mut handler = handler;
        handler.set_configuration(&configuration)?;

        handler.start()?;
        handler.connect()?;
        handler.wait_netif_up()?;

        Ok(Self { handler })
    }

    /// Checks if the Wi-Fi connection is currently on.
    ///
    /// # Returns
    ///
    /// `true` if the connection is on, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if checking the state fails.
    pub fn is_on(&self) -> Result<bool> {
        Ok(self.handler.is_connected()?)
    }
}
