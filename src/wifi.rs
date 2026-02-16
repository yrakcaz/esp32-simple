use anyhow::{anyhow, Result};
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};

/// Wi-Fi network configuration containing SSID, password, and authentication method.
///
/// # Fields
/// * `ssid` - The network SSID.
/// * `password` - The network password.
/// * `auth` - The authentication method (e.g., `WPA2Personal`).
pub struct Config {
    ssid: &'static str,
    password: &'static str,
    auth: AuthMethod,
}

impl Config {
    fn new(ssid: &'static str, password: &'static str, auth: AuthMethod) -> Self {
        Self {
            ssid,
            password,
            auth,
        }
    }

    /// Returns the configured Wi-Fi SSID.
    ///
    /// # Returns
    /// The SSID as a string slice.
    #[must_use]
    pub fn ssid(&self) -> &str {
        self.ssid
    }

    /// Returns the configured Wi-Fi password.
    ///
    /// # Returns
    /// The password as a string slice.
    #[must_use]
    pub fn password(&self) -> &str {
        self.password
    }

    /// Returns the configured authentication method.
    ///
    /// # Returns
    /// The [`AuthMethod`] variant for this configuration.
    #[must_use]
    pub fn auth(&self) -> AuthMethod {
        self.auth
    }

    /// Creates a `Config` from compile-time environment variables.
    ///
    /// Reads `WIFI_SSID` and `WIFI_PASSWORD` via `option_env!` and defaults
    /// to `WPA2Personal` authentication.
    ///
    /// # Returns
    /// A `Config` populated from environment variables.
    ///
    /// # Errors
    /// Returns an error if `WIFI_SSID` or `WIFI_PASSWORD` is not set at compile time.
    pub fn from_env() -> Result<Self> {
        let ssid = option_env!("WIFI_SSID")
            .ok_or_else(|| anyhow!("WIFI_SSID environment variable not set"))?;
        let password = option_env!("WIFI_PASSWORD")
            .ok_or_else(|| anyhow!("WIFI_PASSWORD environment variable not set"))?;

        Ok(Self::new(ssid, password, AuthMethod::WPA2Personal))
    }
}

/// Represents a Wi-Fi connection, handling its configuration and state management.
///
/// This struct leverages the `BlockingWifi` handler from the ESP-IDF framework for managing the connection.
pub struct Connection<'a> {
    handler: BlockingWifi<EspWifi<'a>>,
}

impl<'a> Connection<'a> {
    /// Creates a new `Connection` instance with the given Wi-Fi handler and configuration.
    ///
    /// Configures, starts, connects, and waits for the network interface to come up.
    ///
    /// # Arguments
    ///
    /// * `handler` - The Wi-Fi handler to manage the connection.
    /// * `config` - The Wi-Fi configuration containing SSID, password, and authentication method.
    ///
    /// # Returns
    ///
    /// A connected `Connection` instance ready for use.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be set, SSID/password conversion fails,
    /// or the connection cannot be established.
    pub fn new(handler: BlockingWifi<EspWifi<'a>>, config: &Config) -> Result<Self> {
        let configuration: Configuration =
            Configuration::Client(ClientConfiguration {
                auth_method: config.auth(),
                ssid: config
                    .ssid()
                    .try_into()
                    .map_err(|()| anyhow!("Failed to convert SSID"))?,
                password: config
                    .password()
                    .try_into()
                    .map_err(|()| anyhow!("Failed to convert password"))?,
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
