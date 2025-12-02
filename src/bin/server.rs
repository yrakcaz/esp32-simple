use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};

use esp_layground::{
    http::Client,
    infra::State,
    thread,
    wifi::{Config as WifiConfig, Connection},
};

mod common;
use common::{hw::Context, logic::StateMachine};

const INIT_STATE: State = State::On;

fn main() -> ! {
    thread::main(|| {
        EspLogger::initialize_default();

        // Setup common context (peripherals, threads, etc.) and keep modem for WiFi
        let context = Context::try_default()?;
        let (dispatcher, advertiser, led, led_timer, _, _, _, modem) =
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

        let mut sm = StateMachine::new(
            INIT_STATE.into(),
            dispatcher,
            advertiser,
            led,
            led_timer,
            None, // No GPS for server binary
            Some(http),
        )?;

        sm.run()
    })
}
