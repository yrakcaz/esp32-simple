use esp_idf_svc::log::EspLogger;
use std::sync::{Arc, Mutex};

use esp_layground::{
    gps::{Reading, Sensor},
    infra::{Poller, State},
    thread,
};

mod common;
use common::{hw::Context, logic::StateMachine};

const INIT_STATE: State = State::On;

fn main() -> ! {
    thread::main(|| {
        EspLogger::initialize_default();

        // Setup common context (peripherals, threads, etc.)
        let context = Context::try_default()?;

        // Setup GPS sensor thread (client-specific)
        let location = Arc::new(Mutex::new(None::<Reading>));
        let (
            dispatcher,
            advertiser,
            led,
            led_timer,
            gps_notifier,
            button_state,
            uart_driver,
            _,
        ) = context.into_parts();

        let mut gps = Sensor::new(
            gps_notifier,
            button_state,
            uart_driver,
            Arc::clone(&location),
        );
        thread::spawn(move || gps.poll());

        // Create and run state machine with location
        let mut sm = StateMachine::new(
            INIT_STATE.into(),
            dispatcher,
            advertiser,
            led,
            led_timer,
            Some(location),
            None, // No HTTP client for client binary
        )?;

        sm.run()
    })
}
