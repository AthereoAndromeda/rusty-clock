use esp_hal::{
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::Channel,
        timer::{self, Timer, TimerIFace as _},
    },
    peripherals,
};

use crate::utils::mk_static;

pub(crate) struct ChannelBuilder {
    lstimer0: &'static Timer<'static, LowSpeed>,
    ledc: Ledc<'static>,
}

// TODO: Use singleton pattern
macro_rules! add_channels {
    ($($n:expr), *$(,)?) =>{
        ::paste::paste! {
            impl ChannelBuilder {
                $(
                    pub fn [<channel $n>](&mut self, output: ::esp_hal::gpio::Output<'static>) -> Channel<'static, LowSpeed> {
                        let mut channel = self
                            .ledc
                            .channel(::esp_hal::ledc::channel::Number::[<Channel $n>], output);

                            ::esp_hal::ledc::channel::ChannelIFace::configure(
                                &mut channel,
                                ::esp_hal::ledc::channel::config::Config {
                                    timer: self.lstimer0,
                                    duty_pct: 0,
                                    drive_mode: ::esp_hal::gpio::DriveMode::PushPull,
                                },
                            )
                            .expect(concat!("Failed to configure PWM Channel ", $n));

                        channel
                    }
                )*
            }
        }
    }
}

add_channels!(0);

pub(crate) fn init(ledc: peripherals::LEDC<'static>) -> ChannelBuilder {
    let mut ledc = Ledc::new(ledc);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let lstimer0 = mk_static!(timer::Timer<'static, LowSpeed>; ledc.timer(timer::Number::Timer0));

    defmt::expect!(
        lstimer0.configure(timer::config::Config {
            duty: esp_hal::ledc::timer::config::Duty::Duty5Bit,
            clock_source: esp_hal::ledc::timer::LSClockSource::APBClk,
            frequency: esp_hal::time::Rate::from_khz(24),
        }),
        "Failed to configure PWM Timer"
    );

    ChannelBuilder { lstimer0, ledc }
}
