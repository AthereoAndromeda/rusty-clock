mod listener;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use embassy_time::Timer;
use esp_hal::{
    gpio::Output,
    peripherals::{self},
};
use listener::*;

use crate::{BuzzerOutput, mk_static};

pub static IS_BUZZER_ON: AtomicBool = AtomicBool::new(false);

pub enum BuzzerAction {
    On,
    Off,
    Toggle,
}

impl From<bool> for BuzzerAction {
    fn from(value: bool) -> Self {
        if value { Self::On } else { Self::Off }
    }
}

pub static BUZZER_SIGNAL: Signal<CriticalSectionRawMutex, BuzzerAction> = Signal::new();
pub static TIMER_SIGNAL: Signal<CriticalSectionRawMutex, i32> = Signal::new();

pub type Buzzer = Mutex<CriticalSectionRawMutex, Output<'static>>;

pub async fn init_buzzer(
    spawner: Spawner,
    output_pin: peripherals::GPIO5<'static>,
    button_pin: peripherals::GPIO7<'static>,
) -> &'static Buzzer {
    let buzzer_output = Output::new(
        output_pin,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default()
            .with_drive_strength(esp_hal::gpio::DriveStrength::_5mA)
            .with_pull(esp_hal::gpio::Pull::Down),
    );

    let buzz: &'static Mutex<CriticalSectionRawMutex, Output<'static>> =
        mk_static!(Mutex<CriticalSectionRawMutex, Output<'static>>, Mutex::new(buzzer_output));

    spawner.must_spawn(run(buzz));
    spawner.must_spawn(listen_for_button(button_pin));
    spawner.must_spawn(listen_for_timer());

    // Beep 3 times
    for _ in 0..3 {
        Timer::after_millis(300).await;
        BUZZER_SIGNAL.signal(BuzzerAction::Toggle);
    }

    BUZZER_SIGNAL.signal(BuzzerAction::Off);

    buzz
}

#[embassy_executor::task]
async fn run(output: &'static BuzzerOutput) {
    let mut buzzer_state = IS_BUZZER_ON.load(Ordering::SeqCst);

    loop {
        match BUZZER_SIGNAL.wait().await {
            BuzzerAction::On => {
                output.lock().await.set_high();
                buzzer_state = true;
                IS_BUZZER_ON.store(true, Ordering::SeqCst);
            }
            BuzzerAction::Off => {
                output.lock().await.set_low();
                buzzer_state = false;
                IS_BUZZER_ON.store(false, Ordering::SeqCst);
            }
            BuzzerAction::Toggle => {
                output.lock().await.toggle();
                buzzer_state = !buzzer_state;
                IS_BUZZER_ON.store(buzzer_state, Ordering::SeqCst);
            }
        }
    }
}
