use anyhow::{anyhow, Result};
use esp_idf_svc::log::EspLogger;
use log::info;
use std::sync::{Arc, Mutex};

use esp_layground::{
    gps::{Reading, Sensor},
    infra::{Poller, Switch},
    thread,
};

mod common;
use common::{
    hw::Context,
    logic::{trace_func, Core, DeviceNearby, State, Trigger},
};

// State machine for the client device (GPS tracking, BLE advertising).
struct StateMachine<'a> {
    core: Core<'a>,
    location: Arc<Mutex<Option<Reading>>>,
    max_speed_mps: f32,
}

impl<'a> StateMachine<'a> {
    // Creates a new client state machine.
    fn new(core: Core<'a>, location: Arc<Mutex<Option<Reading>>>) -> Self {
        Self {
            core,
            location,
            max_speed_mps: 0.0,
        }
    }

    // Custom button handler that also resets the max speed.
    fn handle_button_pressed(
        core: &mut Core<'_>,
        max_speed_mps: &mut f32,
    ) -> Result<()> {
        trace_func!();

        if core.state.is_off() {
            *max_speed_mps = 0.0;
            core.state = State::on();
        } else {
            core.state = State::off();
        }

        core.advertiser.toggle()
    }

    // Runs the state machine.
    fn run(&mut self) -> Result<()> {
        let max_speed_mps = &mut self.max_speed_mps;
        let location = &self.location;

        self.core.run(|core, triggers| {
            if core.handle_common_triggers(
                triggers,
                |c| Self::handle_button_pressed(c, max_speed_mps),
                |c| {
                    trace_func!();
                    // Only change state if not already Off or ActiveDeviceNearby
                    if c.state.is_on()
                        && !matches!(c.state, State::On(Some(DeviceNearby::Active)))
                    {
                        c.state = State::On(Some(DeviceNearby::Active));
                    }
                    Ok(())
                },
            )? {
                Ok(())
            } else if triggers.contains(&Trigger::GpsDataAvailable) {
                let mut data = location
                    .lock()
                    .map_err(|e| anyhow!("Mutex lock error: {:?}", e))?;

                if let Some(reading) = data.take() {
                    info!("GPS Reading: {}", reading);
                    if let Some(speed) = reading.speed_mps() {
                        if speed > *max_speed_mps {
                            *max_speed_mps = speed;
                        }
                    }
                    let payload = (*max_speed_mps > 0.0).then(|| {
                        let bytes = max_speed_mps.to_le_bytes().to_vec();
                        let kmph = *max_speed_mps * 3.6;
                        info!(
                            "Advertising {} bytes: {:?} (max_speed: {kmph:.2} km/h)",
                            bytes.len(),
                            bytes
                        );
                        bytes
                    });
                    core.advertiser.set_payload(payload)?;
                }
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
            _,
        ) = context.into_parts();

        let mut gps = Sensor::new(
            gps_notifier,
            &Trigger::GpsDataAvailable,
            button_state,
            uart_driver,
            Arc::clone(&location),
        );
        thread::spawn(move || gps.poll());

        // Create and run state machine with location
        let core = Core::new(State::on(), dispatcher, advertiser, led, led_timer)?;
        let mut sm = StateMachine::new(core, location);

        sm.run()
    })
}
