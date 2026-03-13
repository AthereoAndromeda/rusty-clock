use super::print_lines;
use crate::{lcd::LcdDisplay, rtc_ds3231::TIME_WATCH};
use lcd::Backlight as _;

#[embassy_executor::task]
pub async fn runner_task(mut display: LcdDisplay) -> ! {
    init_display(&mut display).await;
    let mut rx = TIME_WATCH.receiver().unwrap();

    loop {
        let time = rx.changed().await;

        let s = time.local().to_human_short();
        let s = s.split_at(9);
        #[expect(clippy::string_slice, reason = "ASCII/CP437")]
        print_lines(&mut display, s.0, &s.1[2..]).await;
    }
}

async fn init_display(display: &mut LcdDisplay) {
    display.clear().await;

    display
        .init(lcd::FunctionLine::Line2, lcd::FunctionDots::Dots5x10)
        .await;
    display.set_backlight(true).await;

    display
        .display(
            lcd::DisplayMode::DisplayOn,
            lcd::DisplayCursor::CursorOff,
            lcd::DisplayBlink::BlinkOff,
        )
        .await;

    display
        .entry_mode(
            lcd::EntryModeDirection::EntryRight,
            lcd::EntryModeShift::NoShift,
        )
        .await;
}
