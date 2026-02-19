//! # `Buzzer`
//!
//! Holds implementation details of the [`Buzzer`] struct.

use esp_hal::ledc::{LowSpeed, channel::ChannelIFace as _};

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
    pub fn new(output: esp_hal::ledc::channel::Channel<'static, LowSpeed>) -> Self
    where
        Self: Sized,
    {
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

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }
}
