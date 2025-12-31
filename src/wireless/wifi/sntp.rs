use core::net::{IpAddr, SocketAddr};

use defmt::{info, warn};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer, WithTimeout};
use sntpc::NtpContext;

use crate::{NTP_SERVER_ADDR, NTP_SIGNAL, rtc_ds3231::RtcDS3231};

#[derive(Copy, Clone)]
/// Time in us
pub struct SntpTimestamp(u64);

impl sntpc::NtpTimestampGenerator for SntpTimestamp {
    fn init(&mut self) {}

    fn timestamp_sec(&self) -> u64 {
        self.0 / 1_000_000
    }
    fn timestamp_subsec_micros(&self) -> u32 {
        (self.0 % 1_000_000) as u32
    }
}

impl Default for SntpTimestamp {
    fn default() -> Self {
        Self(0)
    }
}

#[embassy_executor::task]
pub async fn fetch_sntp(
    net_stack: embassy_net::Stack<'static>,
    rtc: &'static Mutex<CriticalSectionRawMutex, RtcDS3231>,
) {
    // Create UDP socket
    let mut udp_rx_meta = [PacketMetadata::EMPTY; 16];
    let mut udp_tx_meta = [PacketMetadata::EMPTY; 16];
    let mut udp_tx_buffer = [0u8; 4096];
    let mut udp_rx_buffer = [0u8; 4096];

    let mut udp_socket = UdpSocket::new(
        net_stack,
        &mut udp_rx_meta,
        &mut udp_rx_buffer,
        &mut udp_tx_meta,
        &mut udp_tx_buffer,
    );

    // 123 is SNTP port
    udp_socket.bind(123).unwrap();

    info!("[sntp] Waiting for Network Link...");
    match net_stack
        .wait_link_up()
        .with_timeout(Duration::from_secs(180))
        .await
    {
        Ok(_) => {
            info!("[sntp] Network Link is Up!");
        }
        Err(_) => {
            warn!("[sntp] Network Link Timed Out!");
            return;
        }
    };

    info!("[sntp] Waiting to get IP address...");
    match net_stack
        .wait_config_up()
        .with_timeout(Duration::from_secs(180))
        .await
    {
        Ok(_) => {
            let config = net_stack
                .config_v4()
                .expect("Should be here since we waited for config");
            info!("[sntp] Got IP: {}", config.address);
        }
        Err(_) => {
            warn!("[sntp] DHCP IP Address Request Timed Out!");
            return;
        }
    };

    // TODO: Retry and connect to multiple NTP servers
    let ntp_addrs = net_stack
        .dns_query(NTP_SERVER_ADDR, smoltcp::wire::DnsQueryType::A)
        .await
        .unwrap();

    if ntp_addrs.is_empty() {
        warn!("[sntp] Failed to resolve DNS! Falling back to stored RTC time");
        return;
    }

    info!("[sntp] Sending SNTP Request...");
    let addr: IpAddr = ntp_addrs[0].into();
    let current_timestamp = rtc
        .lock()
        .await
        .datetime()
        .await
        .unwrap()
        .and_utc()
        .timestamp_micros();

    let result = sntpc::get_time(
        SocketAddr::from((addr, 123)),
        &udp_socket,
        NtpContext::new(SntpTimestamp(current_timestamp as u64)),
    )
    .await;

    info!("[sntp] Received a response!");
    match result {
        Ok(time) => {
            info!("[sntp] Response: {:?}", time);
            let jt = jiff::Timestamp::from_second(time.sec() as i64)
                .unwrap()
                .checked_add(
                    jiff::Span::new()
                        .nanoseconds((time.seconds_fraction as i64 * 1_000_000_000) >> 32),
                )
                .unwrap()
                .to_zoned(jiff::tz::TimeZone::fixed(
                    jiff::tz::Offset::from_hours(8).unwrap(),
                ));

            NTP_SIGNAL.signal(jt.timestamp().as_second());

            #[cfg(debug_assertions)]
            {
                // Create a Jiff Timestamp from seconds and nanoseconds
                use crate::EPOCH_SIGNAL;
                let jtf = jt.timestamp().as_second();
                let rtc_time = EPOCH_SIGNAL.wait().await;
                info!("[sntp] ntp: {}", jtf);
                info!("[sntp] rtc: {}", rtc_time);
                info!("[sntp] Difference: {}", jtf - rtc_time);
            }
        }
        Err(e) => {
            warn!("[sntp] Failed to get NTP Time!: {:?}", e);
        }
    }

    info!("[sntp] Task Complete!")
}
