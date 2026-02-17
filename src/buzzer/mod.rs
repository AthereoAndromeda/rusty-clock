//! # Buzzer
//! This module holds all the logic regarding the buzzer.

mod buzzer_struct;
mod listener;

pub(crate) use buzzer_struct::*;
use listener::*;

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;
use esp_hal::{
    ledc::LowSpeed,
    peripherals::{self},
};

pub(crate) enum BuzzerAction {
    On,
    Off,
    Toggle,
}

impl From<bool> for BuzzerAction {
    fn from(value: bool) -> Self {
        if value { Self::On } else { Self::Off }
    }
}

/// Use this to set the buzzer signal
pub(crate) static BUZZER_ACTION_SIGNAL: Signal<CriticalSectionRawMutex, BuzzerAction> =
    Signal::new();

/// Signal is used to set the timer in seconds
pub(crate) static TIMER_SIGNAL: Signal<CriticalSectionRawMutex, u32> = Signal::new();

/// NOTE: ESP32-C3 does not natively support 8-bit atomics (rv32imc).
/// portable_atomic supports fetch_not
pub(crate) static IS_BUZZER_ON: portable_atomic::AtomicBool =
    portable_atomic::AtomicBool::new(false);

pub(crate) static VOLUME_SIGNAL: Signal<CriticalSectionRawMutex, u8> = Signal::new();

/// Initialize the buzzer and beep to signal readiness
pub(super) async fn init(
    spawner: Spawner,
    output_channel: esp_hal::ledc::channel::Channel<'static, LowSpeed>,
    button_pin: peripherals::GPIO7<'static>,
    alarm_pin: peripherals::GPIO6<'static>,
) {
    let mut buzzer = Buzzer::new(output_channel);
    buzzer.set_volume(100);

    spawner.must_spawn(listen_for_action_and_volume(buzzer));
    spawner.must_spawn(listen_for_alarm(alarm_pin));
    spawner.must_spawn(listen_for_button(button_pin));
    spawner.must_spawn(listen_for_timer());

    // Beep 3 times
    for _ in 0..3 {
        Timer::after_millis(300).await;
        BUZZER_ACTION_SIGNAL.signal(BuzzerAction::Toggle);
    }

    BUZZER_ACTION_SIGNAL.signal(BuzzerAction::Off);
}
