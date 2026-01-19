use core::sync::atomic::{AtomicBool, Ordering};

use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, InputConfig, Output, Pull},
    peripherals::{self},
};

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
    let mut buzzer_state: bool = IS_BUZZER_ON.load(Ordering::SeqCst).into();

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

#[embassy_executor::task]
pub async fn listen_for_timer() {
    info!("[buzzer:listen_for_timer] Listening for timer");

    loop {
        let secs = TIMER_SIGNAL.wait().await;
        TIMER_SIGNAL.reset();

        Timer::after_secs(secs as u64).await;
        BUZZER_SIGNAL.signal(BuzzerAction::On);

        // WARNING: Could potentially turn off the prematurely buzzer if
        // an alarm goes off between the interval of waiting
        Timer::after_secs(30).await;
        BUZZER_SIGNAL.signal(BuzzerAction::Off);
    }
}

#[embassy_executor::task]
pub async fn listen_for_button(input_pin: peripherals::GPIO7<'static>) {
    let mut input = Input::new(input_pin, InputConfig::default().with_pull(Pull::Up));

    loop {
        info!("Waiting for alarm button press");
        input.wait_for_falling_edge().await;

        info!("Alarm Button Pressed!");
        BUZZER_SIGNAL.signal(BuzzerAction::Off);
        Timer::after_millis(500).await;
    }
}
