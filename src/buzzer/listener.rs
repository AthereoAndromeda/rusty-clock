use crate::buzzer::{BUZZER_SIGNAL, BuzzerAction, TIMER_SIGNAL};
use defmt::info;
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    peripherals,
};

#[embassy_executor::task]
pub(super) async fn listen_for_timer() {
    info!("[buzzer:listen_for_timer] Listening for timer");

    loop {
        let secs = TIMER_SIGNAL.wait().await;

        Timer::after_secs(secs as u64).await;
        BUZZER_SIGNAL.signal(BuzzerAction::On);

        // WARNING: Could potentially turn off the prematurely buzzer if
        // an alarm goes off between the interval of waiting
        Timer::after_secs(30).await;
        BUZZER_SIGNAL.signal(BuzzerAction::Off);
    }
}

#[embassy_executor::task]
pub(super) async fn listen_for_button(input_pin: peripherals::GPIO7<'static>) {
    let mut input = Input::new(input_pin, InputConfig::default().with_pull(Pull::Up));

    loop {
        info!("Waiting for alarm button press");
        input.wait_for_falling_edge().await;

        info!("Alarm Button Pressed!");
        BUZZER_SIGNAL.signal(BuzzerAction::Off);
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
pub(super) async fn listen_for_alarm(alarm_pin: peripherals::GPIO6<'static>) {
    info!("Initializing Alarm Listener...");
    let mut alarm_input = Input::new(alarm_pin, InputConfig::default().with_pull(Pull::Up));

    loop {
        info!("Waiting for alarm...");
        alarm_input.wait_for_falling_edge().await;

        info!("DS3231 Interrupt Received!");
        BUZZER_SIGNAL.signal(BuzzerAction::On);

        #[cfg(debug_assertions)]
        {
            // Stop it from bleeding my ears while devving
            Timer::after_secs(5).await;
            BUZZER_SIGNAL.signal(BuzzerAction::Off);
            info!("Buzzer set low");
        }
    }
}
