use anyhow::Result;
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
    ble::{self, Advertiser, Scanner},
    button::Button,
    clock::Timer,
    infra::{Poller, State},
    light::Led,
    message::{Dispatcher, Notifier},
    thread::spawn,
};

const BLINK_FREQ_HZ: u64 = 3;
const INIT_STATE: State = State::On;

/// Common hardware context shared by both server and client binaries.
pub struct Context<'a> {
    dispatcher: Dispatcher,
    advertiser: Advertiser,
    led: Led<'a>,
    led_timer: Timer<'a>,
    button_state: Arc<Mutex<State>>,
    uart_driver: UartRxDriver<'a>,
    gps_notifier: Notifier,
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
        ble::initialize_default()?;

        let name = option_env!("APP_NAME").unwrap_or("ESPlayground");

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
        let button_state = Arc::new(Mutex::new(INIT_STATE));

        // Spawn button polling thread
        let mut button =
            Button::new(button_notifier, pin_driver, Arc::clone(&button_state))?;
        spawn(move || button.poll());

        // Spawn BLE scanner thread
        let ble_timer = Timer::new(ble_timer_driver)?;
        let mut scanner =
            Scanner::new(ble_notifier, ble_timer, Arc::clone(&button_state), name)?;
        spawn(move || scanner.poll());

        // Setup LED and advertiser
        let advertiser = Advertiser::new(name, INIT_STATE)?;
        let led = Led::new(tx_rmt_driver)?;
        let mut led_timer = Timer::new(led_timer_driver)?;
        led_timer.configure_interrupt(BLINK_FREQ_HZ, led_timer_notifier)?;

        Ok(Context {
            dispatcher,
            advertiser,
            led,
            led_timer,
            button_state,
            uart_driver,
            gps_notifier,
            modem,
        })
    }

    pub fn into_parts(
        self,
    ) -> (
        Dispatcher,
        Advertiser,
        Led<'a>,
        Timer<'a>,
        Notifier,
        Arc<Mutex<State>>,
        UartRxDriver<'a>,
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
            self.modem,
        )
    }
}
