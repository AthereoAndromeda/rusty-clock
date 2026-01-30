pub(super) mod alarm;
pub(super) mod buzzer;
pub(super) mod time;

use embassy_time::Timer;
use picoserve::response::IntoResponse;

use crate::{
    TIME_WATCH, buzzer::TIMER_SIGNAL, rtc_ds3231::rtc_time::RtcTime, wireless::wifi::sntp::NTP_SYNC,
};

pub struct TimeEvent;

impl picoserve::response::sse::EventSource for TimeEvent {
    async fn write_events<W: picoserve::io::Write>(
        self,
        mut writer: picoserve::response::sse::EventWriter<'_, W>,
    ) -> Result<(), W::Error> {
        let mut anon_recv = TIME_WATCH.anon_receiver();

        loop {
            #[cfg(debug_assertions)]
            defmt::debug!("[sse:time] Writing Event...");

            let time = anon_recv.try_get();
            if time.is_none() {
                writer.write_keepalive().await?;
                Timer::after_secs(1).await;
                continue;
            }

            writer.write_event("time", time.unwrap()).await?;

            #[cfg(debug_assertions)]
            defmt::debug!("[sse:time] Event Written!");
            Timer::after_secs(1).await;
        }
    }
}

impl picoserve::response::sse::EventData for RtcTime {
    async fn write_to<W: picoserve::io::Write>(self, writer: &mut W) -> Result<(), W::Error> {
        writer.write_all(self.to_human_local().as_bytes()).await?;
        Ok(())
    }
}

pub(super) async fn get_help() -> &'static str {
    r#"
Hello from ESP32! This is the web server for rusty clock

All paths use GET unless otherwise specified

Paths:
/                         - Gets Control Panel webpage
/help                     - Prints this help message.
/time                     - Gets current time
/epoch                    - Gets current time as UNIX_EPOCH

/alarm                    - Gets alarm settings
/alarm/:hour/:minute      - Sets alarm
/alarm/off                - Turns off alarm if active
/alarm/on
/alarm/toggle                

/timer                    - Set a timer to buzz
"#
}

pub(super) async fn set_timer(sec: i32) -> impl IntoResponse {
    TIMER_SIGNAL.signal(sec);
}

pub(super) async fn get_sync() -> impl IntoResponse {
    NTP_SYNC.signal(());
}
