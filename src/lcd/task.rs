use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use super::print_lines;
use crate::{i2c::I2cBus, lcd::LcdDisplay, rtc_ds3231::TIME_WATCH};

#[embassy_executor::task]
pub async fn runner_task(
    display: &'static Mutex<CriticalSectionRawMutex, LcdDisplay<I2cBus>>,
) -> ! {
    let mut rx = TIME_WATCH.receiver().unwrap();

    loop {
        let time = rx.changed().await;
        let mut display = display.lock().await;

        let s = time.local().to_human_short();
        let s = s.split_at(9);
        #[expect(clippy::string_slice, reason = "ASCII/CP437")]
        print_lines(&mut display, s.0, &s.1[2..]).await.unwrap();
    }
}
