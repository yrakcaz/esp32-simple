use anyhow::{anyhow, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};
use log::info;
use std::sync::{Arc, Mutex};

use esp_layground::{
    http::Client,
    infra::Switch,
    thread,
    wifi::{Config as WifiConfig, Connection},
};

mod common;
use common::{
    hw::Context,
    logic::{trace_func, Core, DeviceNearby, State},
};

// State machine for the server device (BLE scanning, HTTP posting).
struct StateMachine<'a> {
    core: Core<'a>,
    http: Client<'a>,
    url: &'a str,
    param: &'a str,
    ble_payload: Arc<Mutex<Option<Vec<u8>>>>,
}

impl<'a> StateMachine<'a> {
    // Creates a new server state machine.
    fn new(
        core: Core<'a>,
        http: Client<'a>,
        ble_payload: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<Self> {
        let url = option_env!("HTTP_URL")
            .ok_or_else(|| anyhow!("HTTP_URL environment variable not set"))?;
        let param = option_env!("HTTP_PARAM")
            .ok_or_else(|| anyhow!("HTTP_PARAM environment variable not set"))?;

        Ok(Self {
            core,
            http,
            url,
            param,
            ble_payload,
        })
    }

    // Sends the max speed from BLE payload over HTTP.
    // Does nothing if no payload is available (not an error).
    fn post_speed(
        http: &mut Client<'_>,
        url: &str,
        param: &str,
        ble_payload: &Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<()> {
        let mut data = ble_payload
            .lock()
            .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?;

        match data.take() {
            None => {
                info!("No BLE payload available to post");
                Ok(())
            }
            Some(payload) => {
                let bytes: [u8; 4] =
                    payload.as_slice().try_into().map_err(|_| {
                        anyhow!("Invalid BLE payload length: {}", payload.len())
                    })?;
                let max_speed_mps = f32::from_le_bytes(bytes);
                let max_speed_kmph = max_speed_mps * 3.6;
                info!(
                    "Received BLE payload: {} bytes: {:?} (max_speed: {max_speed_kmph:.2} km/h)",
                    payload.len(),
                    payload
                );

                let url = format!("{url}?{param}={max_speed_kmph:.2}");
                let status = http.post(&url, None)?;
                info!("HTTP POST request sent to {}, status: {}", url, status);

                Ok(())
            }
        }
    }

    // Custom device found active handler that posts speed data.
    fn handle_device_found_active(
        core: &mut Core<'_>,
        http: &mut Client<'_>,
        url: &str,
        param: &str,
        ble_payload: &Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<()> {
        trace_func!();

        if core.state.is_on() {
            // Only post when transitioning to DeviceNearby::Active.
            if !matches!(core.state, State::On(Some(DeviceNearby::Active))) {
                Self::post_speed(http, url, param, ble_payload)?;
            }
            core.state = State::On(Some(DeviceNearby::Active));
        }

        Ok(())
    }

    // Runs the state machine.
    fn run(&mut self) -> Result<()> {
        let http = &mut self.http;
        let url = self.url;
        let param = self.param;
        let ble_payload = &self.ble_payload;

        self.core.run(|core, triggers| {
            if core.handle_common_triggers(
                triggers,
                |c| {
                    trace_func!();
                    c.state = if c.state.is_off() {
                        State::on()
                    } else {
                        State::off()
                    };
                    c.advertiser.toggle()
                },
                |c| {
                    Self::handle_device_found_active(
                        c,
                        http,
                        url,
                        param,
                        ble_payload,
                    )
                },
            )? {
                Ok(())
            } else {
                Err(anyhow!("Unknown triggers: {:?}", triggers))
            }
        })
    }
}

fn main() -> ! {
    thread::main(|| {
        EspLogger::initialize_default();

        // Setup common context (peripherals, threads, etc.) and keep modem for WiFi
        let context = Context::try_default()?;
        let (dispatcher, advertiser, led, led_timer, _, _, _, ble_payload, modem) =
            context.into_parts();

        // Setup WiFi and HTTP client for server
        let nvs = EspDefaultNvsPartition::take()?;
        let sys_loop = EspSystemEventLoop::take()?;

        let wifi_driver = BlockingWifi::wrap(
            EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
            sys_loop,
        )?;

        let wifi_config = WifiConfig::from_env()?;
        let wifi = Connection::new(wifi_driver, &wifi_config)?;
        let http = Client::new(wifi)?;

        let core = Core::new(State::on(), dispatcher, advertiser, led, led_timer)?;
        let mut sm = StateMachine::new(core, http, ble_payload)?;

        sm.run()
    })
}
