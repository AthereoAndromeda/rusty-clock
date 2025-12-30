//! Functions for module sleep
use core::time::Duration;

use defmt::info;

use esp_hal::gpio;
use esp_hal::peripherals::{self, LPWR};
use esp_hal::rtc_cntl::sleep::{RtcioWakeupSource, WakeupLevel};
use esp_hal::rtc_cntl::{Rtc, reset_reason, wakeup_cause};
use esp_hal::system::Cpu;

/// Enter deep sleep for the specified interval
///
/// **NOTE**: WiFi must be turned off before entering deep sleep, otherwise
/// it will block indefinitely.
pub fn enter_deep(rtc_cntl: LPWR, mut p: peripherals::GPIO4) -> ! {
    let pins: &mut [(&mut dyn gpio::RtcPinWithResistors, WakeupLevel)] =
        &mut [(&mut p, WakeupLevel::Low)];

    let wakeup_source = RtcioWakeupSource::new(pins);
    let mut rtc = Rtc::new(rtc_cntl);
    let reason = reset_reason(Cpu::ProCpu).unwrap();
    let wakeup_reason = wakeup_cause();

    info!("Entering deep sleep");
    rtc.sleep_deep(&[&wakeup_source]);
}
