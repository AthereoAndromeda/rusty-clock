use super::{LCD_COMMANDS, print_lines};
use crate::{
    lcd::{LcdAction, LcdDisplay},
    rtc_ds3231::{TIME_WATCH, rtc_time::RtcDateTime},
};
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

    loop {
        let action = select(rx.changed(), LCD_COMMANDS.wait()).await;

        match action {
            Either::First(time) => time_handle(&mut display, time).await,
            Either::Second(action) => action_handle(&mut display, action).await,
        }
    }
}

async fn time_handle(display: &mut LcdDisplay, time: RtcDateTime<Utc>) {
    let s = time.local().to_human_short();
    debug_assert!(s.is_ascii());
    let s = s.split_at(9);
    #[expect(clippy::string_slice, reason = "ASCII/CP437")]
    print_lines(display, s.0, &s.1[2..]).await;
}

async fn action_handle(display: &mut LcdDisplay, action: LcdAction) {
    match action {
        LcdAction::BacklightOn => display.set_backlight(true).await,
        LcdAction::BacklightOff => display.set_backlight(false).await,
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
        .init(lcd::FunctionLine::Line2, lcd::FunctionDots::Dots5x10)
        .await;

    let backlight = env!("ENABLE_LCD").trim() == "1";
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
