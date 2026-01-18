use core::net::{IpAddr, SocketAddr};
use defmt::{debug, info, warn};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use embassy_time::{Duration, WithTimeout};
use sntpc::NtpContext;

use crate::{NTP_SERVER_ADDR, TIME_WATCH, rtc_ds3231::RtcDS3231};

pub static NTP_SYNC: Signal<CriticalSectionRawMutex, ()> = Signal::new();
const SNTP_PORT: u16 = 123;

#[derive(Copy, Clone, Default)]
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

#[embassy_executor::task]
pub async fn fetch_sntp(
    net_stack: embassy_net::Stack<'static>,
    rtc: &'static Mutex<CriticalSectionRawMutex, RtcDS3231>,
) {
    // Create UDP socket
    let mut udp_rx_meta = [PacketMetadata::EMPTY; 16];
    let mut udp_tx_meta = [PacketMetadata::EMPTY; 16];
    let mut udp_tx_buffer = [0u8; 1024];
    let mut udp_rx_buffer = [0u8; 1024];

    let mut udp_socket = UdpSocket::new(
        net_stack,
        &mut udp_rx_meta,
        &mut udp_rx_buffer,
        &mut udp_tx_meta,
        &mut udp_tx_buffer,
    );

    udp_socket.bind(SNTP_PORT).unwrap();

    loop {
        fetch_sntp_inner(net_stack, rtc, &udp_socket).await;
        NTP_SYNC.wait().await;
    }
}

async fn fetch_sntp_inner(
    net_stack: embassy_net::Stack<'static>,
    rtc: &'static Mutex<CriticalSectionRawMutex, RtcDS3231>,
    udp_socket: &UdpSocket<'_>,
) {
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
    let ntp_addrs_response = net_stack
        .dns_query(NTP_SERVER_ADDR, smoltcp::wire::DnsQueryType::A)
        .with_timeout(Duration::from_secs(180))
        .await;

    let ntp_addrs = match ntp_addrs_response {
        Ok(addrs) => addrs.unwrap(),
        Err(_) => {
            warn!("[sntp] DNS Request Timeout!");
            return;
        }
    };

    if ntp_addrs.is_empty() {
        warn!("[sntp] Failed to resolve DNS! Falling back to stored RTC time");
        return;
    }

    let mut recv = TIME_WATCH.receiver().expect("Maximum reached");

    info!("[sntp] Sending SNTP Request...");
    let addr: IpAddr = ntp_addrs[0].into();
    let current_timestamp = recv.get().await.and_utc().timestamp_micros();

    let result = sntpc::get_time(
        SocketAddr::from((addr, SNTP_PORT)),
        udp_socket,
        NtpContext::new(SntpTimestamp(current_timestamp as u64)),
    )
    .await;

    info!("[sntp] Received a response!");
    match result {
        Ok(time) => {
            debug!("[sntp] Response: {:?}", time);
            info!("[rtc:update-timestamp] Setting RTC Datetime to NTP...");

            #[cfg(debug_assertions)]
            {
                use defmt::debug;
                let rtc_time = recv.get().await.and_utc().timestamp();
                debug!("[sntp] NTP: {}", time.seconds);
                debug!("[sntp] RTC: {}", rtc_time);

                let diff = (time.seconds as i64).saturating_sub(rtc_time);
                debug!("[sntp] Difference: {}", diff);
            }

            let datetime = chrono::DateTime::from_timestamp_secs(time.seconds as i64)
                .unwrap()
                .naive_utc();
            rtc.lock().await.set_datetime(&datetime).await.unwrap();
            info!("[rtc:update-timestamp] Succesfully Set RTC Datetime!");
        }
        Err(e) => {
            warn!("[sntp] Failed to get NTP Time!: {:?}", e);
        }
    }

    info!("[sntp] Task Complete!")
}
