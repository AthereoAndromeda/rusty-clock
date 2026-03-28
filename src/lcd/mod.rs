pub(crate) mod error;
mod hardware;
mod task;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use hardware::LcdDisplay;
use hardware::LcdHardware;
use pcf857x::{PcAsync, SlaveAddr};

use crate::i2c::I2cBus;
// use error::LcdDisplayError;

/// The max length of a string that can be displayed in an LCD.
///
/// The length should always be <=40 since the display
/// will overflow to the second line if surpassed.
pub(crate) const MAX_LCD_STRING_LENGTH: usize = 20;

// TEST: Ensure that the maximum is less than 40 to avoid
// accidentally overflowing into the second line
static_assertions::const_assert!(MAX_LCD_STRING_LENGTH <= 40);

/// Simply an alias [`heapless::String`] with a predetermined size.
pub(crate) type LcdDisplayString = heapless::String<MAX_LCD_STRING_LENGTH>;

pub(crate) enum LcdAction {
    BacklightOn,
    BacklightOff,
    BacklightToggle,
    Display(LcdDisplayString),
    DisplayLines(LcdDisplayString, LcdDisplayString),
}

/// The inbox for any LCD Display actions.
pub(crate) static LCD_COMMANDS: Signal<CriticalSectionRawMutex, LcdAction> = Signal::new();

pub fn init(spawner: Spawner, i2c: I2cBus) {
    let hw = LcdHardware::new(PcAsync::new(i2c, SlaveAddr::Alternative(true, true, true)));
    let display: LcdDisplay = lcd::Display::new(hw);
    spawner.must_spawn(task::runner_task(display));
}

/// Prints the two given inputs as two lines.
///
/// # Overflow
/// If more than 40 characters are printed,
/// it will "overflow" to the bottom line.
///
/// This should be handled by the user.
async fn print_lines(display: &mut LcdDisplay, s1: &str, s2: &str) {
    display.clear().await;
    display.print(s1).await;
    display.position(0, 1).await;
    display.print(s2).await;
}
