mod listener;
use listener::*;

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;
use esp_hal::{
    gpio::Output,
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

pub(crate) static BUZZER_SIGNAL: Signal<CriticalSectionRawMutex, BuzzerAction> = Signal::new();
pub(crate) static TIMER_SIGNAL: Signal<CriticalSectionRawMutex, u32> = Signal::new();
/// NOTE: ESP32-C3 does not natively support 8-bit atomics (rv32imc).
/// portable_atomic supports fetch_not
pub(crate) static IS_BUZZER_ON: portable_atomic::AtomicBool =
    portable_atomic::AtomicBool::new(false);

pub(super) async fn init(
    spawner: Spawner,
    output_pin: peripherals::GPIO5<'static>,
    button_pin: peripherals::GPIO7<'static>,
    alarm_pin: peripherals::GPIO6<'static>,
) {
    let buzzer_output = Output::new(
        output_pin,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default()
            .with_drive_strength(esp_hal::gpio::DriveStrength::_5mA)
            .with_pull(esp_hal::gpio::Pull::Down),
    );

    spawner.must_spawn(run(buzzer_output));
    spawner.must_spawn(listen_for_alarm(alarm_pin));
    spawner.must_spawn(listen_for_button(button_pin));
    spawner.must_spawn(listen_for_timer());

    // Beep 3 times
    for _ in 0..3 {
        Timer::after_millis(300).await;
        BUZZER_SIGNAL.signal(BuzzerAction::Toggle);
    }

    BUZZER_SIGNAL.signal(BuzzerAction::Off);
}

#[embassy_executor::task]
async fn run(mut output: Output<'static>) {
    loop {
        match BUZZER_SIGNAL.wait().await {
            BuzzerAction::On => {
                output.set_high();
                IS_BUZZER_ON.store(true, core::sync::atomic::Ordering::Relaxed);
            }
            BuzzerAction::Off => {
                output.set_low();
                IS_BUZZER_ON.store(false, core::sync::atomic::Ordering::Relaxed);
            }
            BuzzerAction::Toggle => {
                output.toggle();
                IS_BUZZER_ON.fetch_not(core::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}
