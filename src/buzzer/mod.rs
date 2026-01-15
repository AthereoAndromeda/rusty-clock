use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, InputConfig, Output, Pull},
    peripherals::{self},
};

use crate::{BuzzerOutput, mk_static};

pub enum BuzzerState {
    On,
    Off,
    Toggle,
}

pub static BUZZER_SIGNAL: Signal<CriticalSectionRawMutex, BuzzerState> = Signal::new();
pub static TIMER_SIGNAL: Signal<CriticalSectionRawMutex, i32> = Signal::new();

pub type Buzzer = Mutex<CriticalSectionRawMutex, Output<'static>>;

pub fn init_buzzer(pin: peripherals::GPIO5<'static>) -> &'static Buzzer {
    let buzzer_output = Output::new(
        pin,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default()
            .with_drive_strength(esp_hal::gpio::DriveStrength::_5mA)
            .with_pull(esp_hal::gpio::Pull::Down),
    );

    let buzz: &'static Mutex<CriticalSectionRawMutex, Output<'static>> =
        mk_static!(Mutex<CriticalSectionRawMutex, Output<'static>>, Mutex::new(buzzer_output));

    buzz
}

#[embassy_executor::task]
pub async fn run(output: &'static BuzzerOutput) {
    loop {
        let state = BUZZER_SIGNAL.wait().await;

        match state {
            BuzzerState::On => {
                output.lock().await.set_high();
            }
            BuzzerState::Off => {
                output.lock().await.set_low();
            }
            BuzzerState::Toggle => {
                output.lock().await.toggle();
            }
        }

        BUZZER_SIGNAL.reset();
    }
}

#[embassy_executor::task]
pub async fn listen_for_timer() {
    info!("[buzzer:listen_for_timer] Listening for timer");

    loop {
        let secs = TIMER_SIGNAL.wait().await;
        TIMER_SIGNAL.reset();

        Timer::after_secs(secs as u64).await;
        BUZZER_SIGNAL.signal(BuzzerState::On);

        // WARNING: Could potentially turn off the prematurely buzzer if
        // an alarm goes off between the interval of waiting
        Timer::after_secs(30).await;
        BUZZER_SIGNAL.signal(BuzzerState::Off);
    }
}

#[embassy_executor::task]
pub async fn listen_for_button(input_pin: peripherals::GPIO7<'static>) {
    let mut input = Input::new(input_pin, InputConfig::default().with_pull(Pull::Up));

    loop {
        info!("Waiting for alarm button press");
        input.wait_for_falling_edge().await;

        info!("Alarm Button Pressed!");
        BUZZER_SIGNAL.signal(BuzzerState::Off);
        Timer::after_millis(500).await;
    }
}
