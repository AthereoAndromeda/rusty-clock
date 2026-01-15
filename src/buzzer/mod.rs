use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    peripherals::{self},
};

use crate::BuzzerOutput;

pub enum BuzzerState {
    On,
    Off,
    Toggle,
}

pub static BUZZER_SIGNAL: Signal<CriticalSectionRawMutex, BuzzerState> = Signal::new();

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
