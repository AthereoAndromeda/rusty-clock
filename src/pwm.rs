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
    lstimer: &'static Timer<'static, LowSpeed>,
    ledc: &'static Ledc<'static>,
    channel: esp_hal::ledc::channel::Number,
}

impl ChannelBuilder {
    pub fn with_output(
        self,
        output: ::esp_hal::gpio::Output<'static>,
    ) -> Channel<'static, LowSpeed> {
        let mut channel = self.ledc.channel(self.channel, output);

        ::esp_hal::ledc::channel::ChannelIFace::configure(
            &mut channel,
            ::esp_hal::ledc::channel::config::Config {
                timer: self.lstimer,
                duty_pct: 0,
                drive_mode: ::esp_hal::gpio::DriveMode::PushPull,
            },
        )
        .expect("Failed to configure PWM Channel ");

        channel
    }
}

macro_rules! add_channels {
    ($($n:expr), *$(,)?) =>{
        ::paste::paste! {
            pub(crate) struct Channels {
                $(pub [<channel $n>]: ChannelBuilder,)*
            }

            fn init_channels(lstimer: &'static Timer<'static, LowSpeed>, ledc: &'static Ledc<'static>) -> Channels {
                $(
                    let [<builder $n>] = ChannelBuilder {
                        lstimer,
                        ledc,
                        channel: ::esp_hal::ledc::channel::Number::[<Channel $n>],
                    };
                )*

                Channels {
                    $([<channel $n>]: [<builder $n>]),*
                }
            }
        }
    }
}

add_channels!(0);

pub(crate) fn init(ledc: peripherals::LEDC<'static>) -> Channels {
    let ledc = mk_static!(Ledc<'static>; Ledc::new(ledc));
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let lstimer0 = mk_static!(timer::Timer<'static, LowSpeed>; ledc.timer(timer::Number::Timer0));

    defmt::expect!(
        lstimer0.configure(timer::config::Config {
            duty: esp_hal::ledc::timer::config::Duty::Duty12Bit,
            clock_source: esp_hal::ledc::timer::LSClockSource::APBClk,
            frequency: esp_hal::time::Rate::from_hz(1000),
        }),
        "Failed to configure PWM Timer"
    );

    init_channels(lstimer0, ledc)
}
