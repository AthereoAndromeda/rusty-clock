use chrono::{Datelike, Timelike};
use ds3231::DS3231;
use embassy_time::Timer;
use heapless::format;

use crate::{I2cAsync, RTC_I2C_ADDR};

#[embassy_executor::task]
pub async fn init_ds3231(i2c: I2cAsync) {
    use ds3231::{
        Alarm1Config, Alarm2Config, Config, DS3231, InterruptControl, Oscillator,
        SquareWaveFrequency, TimeRepresentation,
    };
    // Create configuration
    let config = Config {
        time_representation: TimeRepresentation::TwentyFourHour,
        square_wave_frequency: SquareWaveFrequency::Hz1,
        interrupt_control: InterruptControl::Interrupt,
        battery_backed_square_wave: false,
        oscillator_enable: Oscillator::Enabled,
    };

    // Initialize device
    let mut rtc = DS3231::new(i2c, 0x68);

    // Configure asynchronously
    rtc.configure(&config).await.unwrap();
    // rtc.set_datetime(
    //     &NaiveDate::from_ymd_opt(2025, 12, 22)
    //         .unwrap()
    //         .and_hms_opt(0, 16, 0)
    //         .unwrap(),
    // )
    // .await
    // .unwrap();

    let alarm2 = Alarm2Config::EveryMinute;
    rtc.set_alarm2(&alarm2).await.unwrap();

    loop {
        // Get current date/time asynchronously
        let datetime = rtc.datetime().await.unwrap();
        let time = datetime.time();
        let (hour, minute, second) = (time.hour(), time.minute(), time.second());

        let time_display = format!(10; "{}:{}:{}", hour, minute, second).unwrap();
        // let s = time_display.as_str();
        defmt::info!("{}", time_display);
        Timer::after_secs(1).await;
        // let a: i32 = datetime.time().into()
    }

    // Set alarms asynchronously
    // let alarm1 = Alarm1Config::AtTime {
    //     hours: 9,
    //     minutes: 30,
    //     seconds: 0,
    //     is_pm: None,
    // };
    // rtc.set_alarm1(&alarm1).await.unwrap();
}

#[embassy_executor::task]
pub async fn get_time(i2c: I2cAsync) {
    let mut rtc = DS3231::new(i2c, RTC_I2C_ADDR);

    loop {
        let datetime = rtc.datetime().await.unwrap();
        let date = datetime.date();
        let (year, month, day) = (date.year(), date.month(), date.day());

        let time = datetime.time();
        let (hour, minute, second) = (time.hour(), time.minute(), time.second());
        defmt::info!(
            "{}-{}-{} | {}:{}:{}",
            year,
            month,
            day,
            hour,
            minute,
            second
        );
        Timer::after_secs(1).await;
    }
}
