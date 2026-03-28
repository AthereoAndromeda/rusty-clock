//! # Buzzer
//! This module holds all the logic regarding the buzzer.

mod buzzer_struct;
mod task;

pub(crate) use buzzer_struct::*;

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::{
    ledc::LowSpeed,
    peripherals::{self},
};

pub(crate) enum BuzzerAction {
    On,
    Off,
    Toggle,
    SetVolume(u8),
}

/// Sets the buzzer signal and volume.
pub(crate) static BUZZER_ACTION_SIGNAL: Signal<CriticalSectionRawMutex, BuzzerAction> =
    Signal::new();

/// Signal is used to set the timer in seconds.
pub(crate) static TIMER_SIGNAL: Signal<CriticalSectionRawMutex, u32> = Signal::new();

/// NOTE: ESP32-C3 does not natively support 8-bit atomics (rv32imc).\
/// Hence we use `portable_atomic` since it supports [`fetch_not`](`portable_atomic::AtomicBool::fetch_not`).
pub(crate) static IS_BUZZER_ON: portable_atomic::AtomicBool =
    portable_atomic::AtomicBool::new(false);

/// The volume set for the buzzer. Persists whether the buzzer is on or off.
pub(crate) static BUZZER_VOLUME: portable_atomic::AtomicU8 = portable_atomic::AtomicU8::new(0);

/// Initialize the buzzer and beep to signal readiness.
pub(super) fn init(
    spawner: Spawner,
    output_channel: esp_hal::ledc::channel::Channel<'static, LowSpeed>,
    button_pin: peripherals::GPIO7<'static>,
    alarm_pin: peripherals::GPIO6<'static>,
) {
    let mut buzzer = Buzzer::new(output_channel);
    buzzer.set_volume(80);

    spawner.must_spawn(task::action_task(buzzer));
    spawner.must_spawn(task::alarm_task(alarm_pin));
    spawner.must_spawn(task::button_task(button_pin));
    spawner.must_spawn(task::timer_task());
}
