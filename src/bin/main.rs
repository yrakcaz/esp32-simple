use anyhow::Result;
use esp_idf_hal::{
    gpio::PinDriver,
    prelude::Peripherals,
    rmt::{config::TransmitConfig, TxRmtDriver},
    timer::{TimerConfig, TimerDriver},
};
use esp_idf_svc::log::EspLogger;
#[cfg(feature = "wifi")]
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, EspWifi},
};
use std::sync::{Arc, Mutex};

use esp_layground::{
    ble::{Advertiser, Scanner},
    button::Button,
    clock::Timer,
    infra::{Poller, State},
    light::{Led, BLINK_FREQ},
    logic::StateMachine,
    message::Dispatcher,
    thread::{spawn, ExitGuard},
};
#[cfg(feature = "wifi")]
use esp_layground::{http::Client, wifi::Connection};

fn main() -> Result<()> {
    // main() should never return. Restart the device if it does.
    let _guard = ExitGuard;

    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_hal::sys::link_patches();

    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let ble_timer_peripheral = peripherals.timer01;
    let button_peripheral = peripherals.pins.gpio39;
    let channel_peripheral = peripherals.rmt.channel0;
    let led_peripheral = peripherals.pins.gpio27;
    let led_timer_peripheral = peripherals.timer00;

    let dispatcher = Dispatcher::new()?;
    let ble_notifier = dispatcher.notifier()?;
    let button_notifier = dispatcher.notifier()?;
    let led_timer_notifier = dispatcher.notifier()?;

    let timers_cfg = TimerConfig::new().auto_reload(true);
    let tx_rmt_cfg = TransmitConfig::new().clock_divider(1);

    let ble_timer_driver = TimerDriver::new(ble_timer_peripheral, &timers_cfg)?;
    let led_timer_driver = TimerDriver::new(led_timer_peripheral, &timers_cfg)?;
    let pin_driver = PinDriver::input(button_peripheral)?;
    let tx_rmt_driver =
        TxRmtDriver::new(channel_peripheral, led_peripheral, &tx_rmt_cfg)?;

    // The two inputs to the state machine are the button and the BLE scanner.
    // These inputs are polled in separate threads. However, BLE scanning should
    // not run if the whole system is off. Consequently, the button also needs
    // to be an input to the BLE scanner. This cannot be done using the general
    // dispatcher mechanism because it can have only one listener. Hence, we
    // use a shared state between the button and the BLE scanner.
    let button_state = Arc::new(Mutex::new(State::Off));
    let mut button =
        Button::new(button_notifier, pin_driver, Arc::clone(&button_state))?;
    spawn(move || button.poll());

    let ble_timer = Timer::new(ble_timer_driver)?;
    let mut scanner =
        Scanner::new(ble_notifier, ble_timer, Arc::clone(&button_state))?;
    spawn(move || scanner.poll());

    let advertiser = Advertiser::new()?;
    let led = Led::new(tx_rmt_driver)?;
    let mut led_timer = Timer::new(led_timer_driver)?;
    led_timer.configure_interrupt(BLINK_FREQ, led_timer_notifier)?;

    #[cfg(feature = "wifi")]
    {
        let nvs = EspDefaultNvsPartition::take()?;
        let sys_loop = EspSystemEventLoop::take()?;
        let modem_peripheral = peripherals.modem;
        let wifi_driver = BlockingWifi::wrap(
            EspWifi::new(modem_peripheral, sys_loop.clone(), Some(nvs))?,
            sys_loop,
        )?;

        let wifi = Connection::new(wifi_driver, AuthMethod::WPA2Personal)?;
        let http = Client::new(wifi)?;
        let mut sm =
            StateMachine::new(advertiser, http, led, led_timer, dispatcher)?;
        sm.run()
    }

    #[cfg(not(feature = "wifi"))]
    {
        let mut sm = StateMachine::new(advertiser, led, led_timer, dispatcher)?;
        sm.run()
    }
}
