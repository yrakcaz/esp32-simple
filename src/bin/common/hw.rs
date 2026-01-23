use anyhow::Result;
use esp32_nimble::enums::PowerLevel;
use esp_idf_hal::{
    gpio::{self, PinDriver},
    modem::Modem,
    prelude::Peripherals,
    rmt::{config::TransmitConfig, TxRmtDriver},
    timer::{TimerConfig, TimerDriver},
    uart::{self, UartRxDriver},
    units::Hertz,
};
use std::sync::{Arc, Mutex};

use esp_layground::{
    ble::{self, Advertiser, Scanner, ScannerConfig},
    button::Button,
    clock::Timer,
    infra::{Poller, State},
    light::Led,
    message::{Dispatcher, Notifier},
    thread::spawn,
};

use super::logic::Trigger;

const BLE_ACTIVE_SUFFIX: &str = "-Active";
const BLE_INACTIVE_SUFFIX: &str = "-Inactive";
const BLE_POWER_LEVEL: PowerLevel = PowerLevel::N0;
const BLE_SCAN_FREQ_HZ: u64 = 1;
const BLINK_FREQ_HZ: u64 = 3;

/// Common hardware context shared by both server and client binaries.
pub struct Context<'a> {
    dispatcher: Dispatcher<Trigger>,
    advertiser: Advertiser,
    led: Led<'a>,
    led_timer: Timer<'a, Trigger>,
    button_state: Arc<Mutex<State>>,
    uart_driver: UartRxDriver<'a>,
    gps_notifier: Notifier<Trigger>,
    ble_payload: Arc<Mutex<Option<Vec<u8>>>>,
    modem: Modem,
}

impl<'a> Context<'a> {
    /// Initializes all hardware peripherals and background threads.
    ///
    /// This method initializes:
    /// - System patches and logging
    /// - BLE functionality
    /// - All peripherals (button, BLE scanner, GPS, LED, timers)
    /// - Background threads for button polling, BLE scanning, and GPS reading
    ///
    /// # Errors
    /// Returns an error if any initialization or setup fails.
    pub fn try_default() -> Result<Context<'a>> {
        // It is necessary to call this function once. Otherwise some patches to the runtime
        // implemented by esp-idf-sys might not link properly.
        esp_idf_hal::sys::link_patches();
        ble::initialize(BLE_POWER_LEVEL)?;

        let peripherals = Peripherals::take()?;
        let Peripherals {
            timer01: ble_timer_peripheral,
            timer00: led_timer_peripheral,
            pins,
            rmt,
            uart2: uart_peripheral,
            modem,
            ..
        } = peripherals;
        let button_peripheral = pins.gpio39;
        let channel_peripheral = rmt.channel0;
        let led_peripheral = pins.gpio27;
        let uart_rx = pins.gpio22;

        let dispatcher = Dispatcher::new()?;
        let ble_notifier = dispatcher.notifier()?;
        let button_notifier = dispatcher.notifier()?;
        let led_timer_notifier = dispatcher.notifier()?;
        let gps_notifier = dispatcher.notifier()?;

        let timers_cfg = TimerConfig::new().auto_reload(true);
        let tx_rmt_cfg = TransmitConfig::new().clock_divider(1);
        let uart_cfg = uart::config::Config::new().baudrate(Hertz(115_200));

        let ble_timer_driver = TimerDriver::new(ble_timer_peripheral, &timers_cfg)?;
        let led_timer_driver = TimerDriver::new(led_timer_peripheral, &timers_cfg)?;
        let pin_driver = PinDriver::input(button_peripheral)?;
        let tx_rmt_driver =
            TxRmtDriver::new(channel_peripheral, led_peripheral, &tx_rmt_cfg)?;
        let uart_driver = UartRxDriver::new(
            uart_peripheral,
            uart_rx,
            None::<gpio::AnyIOPin>,
            None::<gpio::AnyIOPin>,
            &uart_cfg,
        )?;

        // Shared state between button and BLE scanner to control scanning based on system state.
        let button_state = Arc::new(Mutex::new(State::on()));
        let ble_payload = Arc::new(Mutex::new(None::<Vec<u8>>));

        // Spawn button polling thread
        let mut button = Button::new(
            button_notifier,
            &Trigger::ButtonPressed,
            pin_driver,
            Arc::clone(&button_state),
        )?;
        spawn(move || button.poll());

        // Spawn BLE scanner thread
        let ble_timer = Timer::new(ble_timer_driver)?;
        let scanner_config = ScannerConfig::new(
            |name| match name {
                n if n.ends_with(BLE_ACTIVE_SUFFIX) => {
                    Some(&Trigger::DeviceFoundActive)
                }
                n if n.ends_with(BLE_INACTIVE_SUFFIX) => {
                    Some(&Trigger::DeviceFoundInactive)
                }
                _ => None,
            },
            &Trigger::DeviceNotFound,
            &Trigger::DeviceFoundActive,
            BLE_SCAN_FREQ_HZ,
        );
        let mut scanner = Scanner::new(
            ble_notifier,
            ble_timer,
            Arc::clone(&button_state),
            Arc::clone(&ble_payload),
            scanner_config,
        )?;
        spawn(move || scanner.poll());

        // Setup BLE advertiser
        let advertiser = Advertiser::new(State::on(), |state, payload| {
            let app_name = option_env!("APP_NAME").unwrap_or("ESPlayground");
            match state {
                State::On(_) => (
                    format!("{app_name}{BLE_ACTIVE_SUFFIX}"),
                    payload.map(|p| p.to_vec()),
                ),
                State::Off => (format!("{app_name}{BLE_INACTIVE_SUFFIX}"), None),
            }
        })?;

        // Setup LED and its timer
        let led = Led::new(tx_rmt_driver)?;
        let mut led_timer = Timer::new(led_timer_driver)?;
        led_timer.configure_interrupt(
            BLINK_FREQ_HZ,
            led_timer_notifier,
            &Trigger::TimerTicked,
        )?;

        Ok(Context {
            dispatcher,
            advertiser,
            led,
            led_timer,
            button_state,
            uart_driver,
            gps_notifier,
            ble_payload,
            modem,
        })
    }

    pub fn into_parts(
        self,
    ) -> (
        Dispatcher<Trigger>,
        Advertiser,
        Led<'a>,
        Timer<'a, Trigger>,
        Notifier<Trigger>,
        Arc<Mutex<State>>,
        UartRxDriver<'a>,
        Arc<Mutex<Option<Vec<u8>>>>,
        Modem,
    ) {
        (
            self.dispatcher,
            self.advertiser,
            self.led,
            self.led_timer,
            self.gps_notifier,
            self.button_state,
            self.uart_driver,
            self.ble_payload,
            self.modem,
        )
    }
}
