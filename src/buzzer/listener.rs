use crate::buzzer::{Buzzer, VOLUME_SIGNAL};

use super::{BUZZER_ACTION_SIGNAL, BuzzerAction, IS_BUZZER_ON, TIMER_SIGNAL};
use defmt::{debug, info};
use embassy_futures::select;
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    peripherals,
};

#[embassy_executor::task]
/// This task handles two functions:
///
/// - Listens for [`BUZZER_ACTION_SIGNAL`] and sets buzzer to
/// the appropriate action.
///
/// - Listens to [`VOLUME_SIGNAL`] and sets the appropriate duty cycle.
///
/// These functions are combined so that we can take ownership of [`Buzzer`] as opposed
/// to wrapping it in a [`Mutex`](`embassy_sync::mutex::Mutex`) to share it between tasks.
pub(super) async fn listen_for_action_and_volume(mut output: Buzzer) -> ! {
    loop {
        let task = select::select(BUZZER_ACTION_SIGNAL.wait(), VOLUME_SIGNAL.wait()).await;

        match task {
            select::Either::First(action) => handle_buzzer_action(&mut output, action),
            select::Either::Second(volume) => output.set_volume(volume),
        }
    }
}

#[inline]
fn handle_buzzer_action(output: &mut Buzzer, action: BuzzerAction) {
    match action {
        BuzzerAction::On => {
            output.activate();
            IS_BUZZER_ON.store(true, core::sync::atomic::Ordering::Relaxed);
        }
        BuzzerAction::Off => {
            output.deactivate();
            IS_BUZZER_ON.store(false, core::sync::atomic::Ordering::Relaxed);
        }
        BuzzerAction::Toggle => {
            output.toggle();
            IS_BUZZER_ON.fetch_not(core::sync::atomic::Ordering::Relaxed);
        }
    }
}

#[embassy_executor::task]
/// Listens for [`TIMER_SIGNAL`] and sets timer accordingly.
pub(super) async fn listen_for_timer() -> ! {
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
/// Listens for a button press which sets buzzer low.
pub(super) async fn listen_for_button(input_pin: peripherals::GPIO7<'static>) -> ! {
    let mut input = Input::new(input_pin, InputConfig::default().with_pull(Pull::Up));

    loop {
        #[cfg(debug_assertions)]
        debug!("Waiting for alarm button press");
        input.wait_for_falling_edge().await;

        debug!("Alarm Button Pressed!");
        BUZZER_ACTION_SIGNAL.signal(BuzzerAction::Off);
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
/// Listen for the alarm interrupt from DS3231 RTC.
pub(super) async fn listen_for_alarm(alarm_pin: peripherals::GPIO6<'static>) -> ! {
    info!("Initializing Alarm Listener...");
    let mut alarm_input = Input::new(alarm_pin, InputConfig::default().with_pull(Pull::Up));

    loop {
        #[cfg(debug_assertions)]
        debug!("Waiting for alarm...");
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
