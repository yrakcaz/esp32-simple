use anyhow::{anyhow, Result};
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};

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

    #[must_use]
    pub fn ssid(&self) -> &str {
        self.ssid
    }

    #[must_use]
    pub fn password(&self) -> &str {
        self.password
    }

    #[must_use]
    pub fn auth(&self) -> AuthMethod {
        self.auth
    }

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
    /// # Arguments
    ///
    /// * `handler` - The Wi-Fi handler to manage the connection.
    /// * `config` - The Wi-Fi configuration containing SSID, password, and authentication method.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be set or if the SSID/password conversion fails.
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
