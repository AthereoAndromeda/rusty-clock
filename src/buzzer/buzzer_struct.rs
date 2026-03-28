//! # `Buzzer`
//!
//! Holds implementation details of the [`Buzzer`] struct.

use esp_hal::ledc::{LowSpeed, channel::ChannelIFace as _};

use crate::buzzer::BUZZER_VOLUME;

/// The [`Buzzer`] can only be `On` or `Off`.
///
/// Volume levels are handled differently.
pub(crate) enum BuzzerState {
    On,
    Off,
}

/// Represents the buzzer with adjustable volume levels.
pub(crate) struct Buzzer {
    output: esp_hal::ledc::channel::Channel<'static, LowSpeed>,
    volume: u8,
    state: BuzzerState,
}

impl Buzzer {
    /// Create a new [`Buzzer`] instance. Starts with:
    /// - Volume at 0.
    /// - State is off.
    pub fn new(output: esp_hal::ledc::channel::Channel<'static, LowSpeed>) -> Self {
        Self {
            output,
            volume: 0,
            state: BuzzerState::Off,
        }
    }

    pub fn activate(&mut self) {
        self.output.set_duty(self.volume).unwrap();
        self.state = BuzzerState::On;
    }

    pub fn deactivate(&mut self) {
        self.output.set_duty(0).unwrap();
        self.state = BuzzerState::Off;
    }

    pub fn toggle(&mut self) {
        match self.state {
            BuzzerState::On => {
                self.deactivate();
            }
            BuzzerState::Off => {
                self.activate();
            }
        }
    }

    /// Sets the duty cycle of the PWM signal.
    ///
    /// # Panics
    /// Panics if `volume > 100`.
    pub fn set_volume(&mut self, volume: u8) {
        defmt::assert!(volume <= 100);
        BUZZER_VOLUME.store(volume, core::sync::atomic::Ordering::Release);
        self.volume = volume;
    }
}
