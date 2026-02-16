use anyhow::{ensure, Result};
use embedded_svc::{http::client::Client as HttpClient, io::Write};
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};

use crate::wifi::Connection;

/// Represents an HTTP client that interacts with a server over Wi-Fi.
///
/// This struct provides methods to send HTTP requests, such as POST requests, using the ESP-IDF framework.
/// It owns an active Wi-Fi connection for the duration of its lifetime.
pub struct Client<'a> {
    client: HttpClient<EspHttpConnection>,
    wifi: Connection<'a>,
}

impl<'a> Client<'a> {
    /// Creates a new `Client` instance with the given Wi-Fi connection.
    ///
    /// # Arguments
    ///
    /// * `wifi` - An active Wi-Fi connection.
    ///
    /// # Returns
    ///
    /// A new `Client` ready to send HTTP requests.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be initialized.
    pub fn new(wifi: Connection<'a>) -> Result<Self> {
        let client =
            HttpClient::wrap(EspHttpConnection::new(&Configuration::default())?);
        Ok(Self { client, wifi })
    }

    /// Sends a POST request to the specified URL with an optional payload.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to send the POST request to.
    /// * `payload` - An optional byte slice containing the payload to send.
    ///
    /// # Returns
    ///
    /// The HTTP status code of the response.
    ///
    /// # Errors
    ///
    /// Returns an error if the Wi-Fi is not connected, the request fails, or the response status is not in the success range.
    pub fn post(&mut self, url: &str, payload: Option<&[u8]>) -> Result<u16> {
        ensure!(self.wifi.is_on()?, "WIFI is off");

        let payload = payload.unwrap_or(b"");
        let content_length_header = format!("{}", payload.len());
        let headers = [
            ("content-type", "text/plain"),
            ("content-length", &*content_length_header),
        ];

        let mut request = self.client.post(url, &headers)?;
        request.write_all(payload)?;
        request.flush()?;

        let response = request.submit()?;
        let status = response.status();
        ensure!(
            (200..300).contains(&status),
            "Request failed with status: {}",
            status
        );

        Ok(status)
    }
}
