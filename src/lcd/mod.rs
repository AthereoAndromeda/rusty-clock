use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embedded_hal_async::i2c::I2c;

pub(crate) mod error;
mod hardware;
mod task;
use hardware::LcdHardware;
use lcd::Backlight as _;
use pcf857x::{PcAsync, SlaveAddr};

use crate::{i2c::I2cBus, utils::mk_static};
use error::LcdDisplayError;

type LcdDisplay<B> = lcd::Display<LcdHardware<B>>;
type LcdDisplayMutex = Mutex<CriticalSectionRawMutex, LcdDisplay<I2cBus>>;

pub async fn init(spawner: Spawner, i2c: I2cBus) {
    let hw = LcdHardware::new(PcAsync::new(i2c, SlaveAddr::Alternative(true, true, true)));
    let mut display: LcdDisplay<I2cBus> = lcd::Display::new(hw);
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

    let display_mutex: &'static LcdDisplayMutex =
        mk_static!(Mutex<CriticalSectionRawMutex, LcdDisplay<I2cBus>>; Mutex::new(display));

    spawner.must_spawn(task::runner_task(display_mutex));
}

/// Prints the two given inputs as two lines.
///
/// The way `HD44780` determines which line to print on is based on
/// how many bytes we printed.
/// If more than 40 characters are printed,
/// it will "overflow" to the bottom line.
///
/// # Errors
/// This will return an error if `s1.len() > 40`.
async fn print_lines(
    display: &mut LcdDisplay<impl I2c>,
    s1: &str,
    s2: &str,
) -> Result<(), LcdDisplayError> {
    // defmt::assert!(s1.len() <= 40, "String in LCD must be less than 40");
    if s1.len() > 40 {
        return Err(LcdDisplayError::OverflowingLines);
    }

    display.clear().await;
    display.print(s1).await;
    display.position(0, 1).await;
    display.print(s2).await;
    Ok(())
}
