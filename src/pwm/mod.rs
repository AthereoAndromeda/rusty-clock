use esp_hal::{
    gpio::Output,
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self, Channel, ChannelIFace},
        timer::{self, Timer, TimerIFace},
    },
    peripherals,
};

use crate::mk_static;

pub(crate) struct ChannelBuilder {
    lstimer0: &'static Timer<'static, LowSpeed>,
    ledc: Ledc<'static>,
    out: Output<'static>,
}

impl ChannelBuilder {
    pub fn channel0(self) -> Channel<'static, LowSpeed> {
        let mut channel0 = self
            .ledc
            .channel(esp_hal::ledc::channel::Number::Channel0, self.out);

        ChannelIFace::configure(
            &mut channel0,
            channel::config::Config {
                timer: self.lstimer0,
                duty_pct: 0,
                drive_mode: esp_hal::gpio::DriveMode::PushPull,
            },
        )
        .unwrap();

        channel0
    }
}

pub(crate) fn init(ledc: peripherals::LEDC<'static>, out: Output<'static>) -> ChannelBuilder {
    let mut ledc = Ledc::new(ledc);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let lstimer0 = mk_static!(timer::Timer<'static, LowSpeed>; ledc.timer(timer::Number::Timer0));
    // let mut lstimer0 = ledc.timer::<LowSpeed>(esp_hal::ledc::timer::Number::Timer0);
    TimerIFace::configure(
        lstimer0,
        timer::config::Config {
            duty: esp_hal::ledc::timer::config::Duty::Duty5Bit,
            clock_source: esp_hal::ledc::timer::LSClockSource::APBClk,
            frequency: esp_hal::time::Rate::from_khz(24),
        },
    )
    .unwrap();

    ChannelBuilder {
        lstimer0,
        ledc,
        out,
    }
}
