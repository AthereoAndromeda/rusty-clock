use crate::buzzer::VOLUME_SIGNAL;

use super::{BUZZER_ACTION_SIGNAL, BuzzerAction, IS_BUZZER_ON, TIMER_SIGNAL};
use defmt::{debug, info};
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    peripherals,
};

#[embassy_executor::task]
/// Listens for [`BUZZER_ACTION_SIGNAL`] and sets buzzer to
/// the appropriate action
pub(super) async fn listen_for_action(output: &'static super::BuzzerMutex) {
    loop {
        match BUZZER_ACTION_SIGNAL.wait().await {
            BuzzerAction::On => {
                output.lock().await.activate();
                IS_BUZZER_ON.store(true, core::sync::atomic::Ordering::Relaxed);
            }
            BuzzerAction::Off => {
                output.lock().await.deactivate();
                IS_BUZZER_ON.store(false, core::sync::atomic::Ordering::Relaxed);
            }
            BuzzerAction::Toggle => {
                output.lock().await.toggle();
                IS_BUZZER_ON.fetch_not(core::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}

#[embassy_executor::task]
/// Listens for [`TIMER_SIGNAL`] and sets timer accordingly
pub(super) async fn listen_for_timer() {
    info!("[buzzer:listen_for_timer] Listening for timer");

    loop {
        let secs = TIMER_SIGNAL.wait().await;

        Timer::after_secs(secs.into()).await;
        BUZZER_ACTION_SIGNAL.signal(BuzzerAction::On);

        // WARNING: Could potentially turn off the prematurely buzzer if
        // an alarm goes off between the interval of waiting
        Timer::after_secs(30).await;
        BUZZER_ACTION_SIGNAL.signal(BuzzerAction::Off);
    }
}

#[embassy_executor::task]
/// Listens for a button press which sets buzzer low
pub(super) async fn listen_for_button(input_pin: peripherals::GPIO7<'static>) {
    let mut input = Input::new(input_pin, InputConfig::default().with_pull(Pull::Up));

    loop {
        info!("Waiting for alarm button press");
        input.wait_for_falling_edge().await;

        info!("Alarm Button Pressed!");
        BUZZER_ACTION_SIGNAL.signal(BuzzerAction::Off);
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
/// Listen for the alarm interrupt from DS3231 RTC
pub(super) async fn listen_for_alarm(alarm_pin: peripherals::GPIO6<'static>) {
    info!("Initializing Alarm Listener...");
    let mut alarm_input = Input::new(alarm_pin, InputConfig::default().with_pull(Pull::Up));

    loop {
        info!("Waiting for alarm...");
        alarm_input.wait_for_falling_edge().await;

        info!("DS3231 Interrupt Received!");
        BUZZER_ACTION_SIGNAL.signal(BuzzerAction::On);

        #[cfg(debug_assertions)]
        {
            // Stop it from bleeding my ears while devving
            Timer::after_secs(5).await;
            BUZZER_ACTION_SIGNAL.signal(BuzzerAction::Off);
            info!("Buzzer set low");
        }
    }
}

#[embassy_executor::task]
pub(super) async fn listen_for_volume(output: &'static super::BuzzerMutex) {
    loop {
        let volume = VOLUME_SIGNAL.wait().await;
        debug!("Volume signal received");
        output.lock().await.set_volume(volume);
    }
}
