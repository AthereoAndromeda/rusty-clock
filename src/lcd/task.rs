use super::{LCD_COMMANDS, LcdAction, LcdDisplay, print_lines};
use crate::rtc_ds3231::{TIME_WATCH, rtc_time::RtcDateTime};
use chrono::Utc;
use embassy_futures::select::{Either, select};
use lcd::Backlight as _;

const LCD_INITIAL: bool = {
    1 == u8::from_str_radix(env!("ENABLE_LCD"), 10)
        .ok()
        .expect("Invalid `ENABLE_LCD`")
};
pub(crate) static BACKLIGHT_STATUS: portable_atomic::AtomicBool =
    portable_atomic::AtomicBool::new(LCD_INITIAL);

#[embassy_executor::task]
pub(super) async fn runner_task(mut display: LcdDisplay) -> ! {
    init_display(&mut display).await;
    let mut rx = TIME_WATCH.receiver().unwrap();
    let mut cached_date_str: heapless::String<11> = heapless::String::new();

    loop {
        let action = select(rx.changed(), LCD_COMMANDS.wait()).await;

        match action {
            Either::First(time) => time_handle(&mut display, time, &mut cached_date_str).await,
            Either::Second(action) => action_handle(&mut display, action).await,
        }
    }
}

async fn time_handle(
    display: &mut LcdDisplay,
    datetime: RtcDateTime<Utc>,
    cached_date_str: &mut heapless::String<11>,
) {
    let s = datetime.local().to_human_short();
    defmt::debug_assert!(s.is_ascii(), "Must be ASCII or CP437 to slice properly");

    let (time_str, date_str) = s.split_at(9);

    #[expect(clippy::string_slice, reason = "ASCII/CP437")]
    // Trims off the separator bar
    let date_str = &date_str[2..];

    // If date is same as cached, print only the time. This avoids stutter
    if date_str == cached_date_str {
        display.home().await;
        display.print(time_str).await;
    } else {
        cached_date_str.clear();
        cached_date_str.push_str(date_str).unwrap();
        print_lines(display, time_str, date_str).await;
    }
}

async fn action_handle(display: &mut LcdDisplay, action: LcdAction) {
    match action {
        LcdAction::BacklightOn => {
            BACKLIGHT_STATUS.store(true, core::sync::atomic::Ordering::Release);
            display.set_backlight(true).await;
        }
        LcdAction::BacklightOff => {
            BACKLIGHT_STATUS.store(false, core::sync::atomic::Ordering::Release);
            display.set_backlight(false).await;
        }
        LcdAction::BacklightToggle => {
            let status = BACKLIGHT_STATUS.fetch_not(core::sync::atomic::Ordering::AcqRel);
            display.set_backlight(!status).await;
        }
        LcdAction::Display(s) => {
            display.print(s.as_str()).await;
        }
        LcdAction::DisplayLines(s1, s2) => print_lines(display, s1.as_str(), s2.as_str()).await,
    }
}

async fn init_display(display: &mut LcdDisplay) {
    display.clear().await;

    display
        .init(lcd::FunctionLine::Line2, lcd::FunctionDots::Dots5x8)
        .await;

    let backlight = const {
        1 == u8::from_str_radix(env!("ENABLE_LCD"), 10)
            .ok()
            .expect("Cannot parse `ENABLE_LCD` env var")
    };

    display.set_backlight(backlight).await;

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
