pub(crate) mod error;
mod hardware;
mod task;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use hardware::LcdHardware;
use pcf857x::{PcAsync, SlaveAddr};

use crate::i2c::I2cBus;
// use error::LcdDisplayError;

const MAX_STRING_LENGTH: usize = 40;

pub(crate) enum LcdAction {
    BacklightOn,
    BacklightOff,
    BacklightToggle,
    Display(heapless::String<MAX_STRING_LENGTH>),
    DisplayLines(
        heapless::String<MAX_STRING_LENGTH>,
        heapless::String<MAX_STRING_LENGTH>,
    ),
}

/// Controls the LCD Backlight.
pub(crate) static LCD_COMMANDS: Signal<CriticalSectionRawMutex, LcdAction> = Signal::new();

type LcdDisplay = lcd::Display<LcdHardware<I2cBus>>;

pub fn init(spawner: Spawner, i2c: I2cBus) {
    let hw = LcdHardware::new(PcAsync::new(i2c, SlaveAddr::Alternative(true, true, true)));
    let display: LcdDisplay = lcd::Display::new(hw);
    spawner.must_spawn(task::runner_task(display));
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
async fn print_lines(display: &mut LcdDisplay, s1: &str, s2: &str) {
    defmt::assert!(s1.len() <= 40, "String in LCD must be less than 40");
    display.clear().await;
    display.print(s1).await;
    display.position(0, 1).await;
    display.print(s2).await;
}
