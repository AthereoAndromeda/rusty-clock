use chrono::Utc;
use core::net::SocketAddr;
use defmt::{debug, info, warn};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, WithTimeout as _};
use explicit_cast::Truncate as _;
use sntpc::NtpContext;
use sntpc_net_embassy::UdpSocketWrapper;
use static_cell::ConstStaticCell;

use crate::rtc_ds3231::{SET_DATETIME_SIGNAL, TIME_WATCH, rtc_time::RtcDateTime};

pub(crate) static NTP_SYNC: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Default NTP server to ping.
const NTP_SERVER_ADDR: &str = option_env!("NTP_SERVER_ADDR").unwrap_or("pool.ntp.org");

const NTP_SERVER_PORT: u16 = {
    let port = option_env!("SNTP_PORT").unwrap_or("123");
    u16::from_str_radix(port, 10)
        .ok()
        .expect("Failed to parse .env: SNTP_PORT")
};

#[derive(Copy, Clone, Default)]
/// Time in us.
struct SntpTimestamp(u64);

impl sntpc::NtpTimestampGenerator for SntpTimestamp {
    fn init(&mut self) {}

    fn timestamp_sec(&self) -> u64 {
        self.0 / 1_000_000
    }
    fn timestamp_subsec_micros(&self) -> u32 {
        (self.0 % 1_000_000).truncate()
    }
}

#[embassy_executor::task]
// Task should only be spawned once
pub(crate) async fn fetch_sntp(net_stack: embassy_net::Stack<'static>) -> ! {
    // NOTE: Using `ConstStaticCell` means these buffers are stored in .bss, thus does
    // not take up any flash space.
    static UDP_RX_META: ConstStaticCell<[PacketMetadata; 16]> =
        ConstStaticCell::new([PacketMetadata::EMPTY; _]);
    static UDP_TX_META: ConstStaticCell<[PacketMetadata; 16]> =
        ConstStaticCell::new([PacketMetadata::EMPTY; _]);
    static UDP_RX_BUFFER: ConstStaticCell<[u8; 1024]> = ConstStaticCell::new([0; _]);
    static UDP_TX_BUFFER: ConstStaticCell<[u8; 1024]> = ConstStaticCell::new([0; _]);

    let udp_rx_meta = UDP_RX_META.take();
    let udp_tx_meta = UDP_TX_META.take();
    let udp_tx_buffer = UDP_TX_BUFFER.take();
    let udp_rx_buffer = UDP_RX_BUFFER.take();

    let mut udp_socket = UdpSocket::new(
        net_stack,
        udp_rx_meta,
        udp_rx_buffer,
        udp_tx_meta,
        udp_tx_buffer,
    );

    udp_socket.bind(NTP_SERVER_PORT).unwrap();
    let wrapper = UdpSocketWrapper::new(udp_socket);

    loop {
        fetch_sntp_inner(net_stack, &wrapper).await;
        NTP_SYNC.wait().await;
    }
}

async fn fetch_sntp_inner(
    net_stack: embassy_net::Stack<'static>,
    udp_socket: &UdpSocketWrapper<'_>,
) {
    info!("[sntp] Waiting for Network Link...");
    if let Ok(()) = net_stack
        .wait_link_up()
        .with_timeout(Duration::from_secs(180))
        .await
    {
        info!("[sntp] Network Link is Up!");
    } else {
        warn!("[sntp] Network Link Timed Out!");
        return;
    }

    info!("[sntp] Waiting to get IP address...");

    if let Ok(()) = net_stack
        .wait_config_up()
        .with_timeout(Duration::from_secs(180))
        .await
    {
        // SAFETY: We just awaited for the config to be up
        unsafe {
            let config = net_stack.config_v4().unwrap_unchecked();
            info!("[sntp] Got IP: {}", config.address);
        }
    } else {
        warn!("[sntp] DHCP IP Address Request Timed Out!");
        return;
    }

    let addr = match super::dns::resolve(NTP_SERVER_ADDR, net_stack).await {
        Ok(addrs) => addrs,
        Err(err) => {
            warn!("[sntp] DNS Error Received: {}", err);
            return;
        }
    };

    let mut recv = TIME_WATCH
        .receiver()
        .expect("[sntp] Max `TIME_WATCH` rx reached");

    info!("[sntp] Sending SNTP Request...");
    let current_timestamp = recv.get().await.timestamp_micros();

    let result = sntpc::get_time(
        SocketAddr::from((addr, NTP_SERVER_PORT)),
        udp_socket,
        NtpContext::new(SntpTimestamp(current_timestamp.cast_unsigned())),
    )
    .await;

    info!("[sntp] Received a response!");
    match result {
        Ok(time) => {
            #[cfg(debug_assertions)]
            debug!("[sntp] Response: {}", time);

            info!("[sntp:update-timestamp] Setting RTC Datetime to NTP...");
            let datetime = RtcDateTime::<Utc>::from_timestamp(time.seconds.into());
            SET_DATETIME_SIGNAL.signal(datetime);

            #[cfg(debug_assertions)]
            {
                use defmt::debug;
                let rtc_time = recv.get().await.to_timestamp();
                debug!("[sntp] NTP: {=u32}", time.seconds);
                debug!("[sntp] RTC: {=u64}", rtc_time);

                let diff = u64::from(time.seconds).saturating_sub(rtc_time);
                debug!("[sntp] Difference: {=u64}", diff);
            }

            info!("[sntp:update-timestamp] Succesfully Set RTC Datetime!");
        }
        Err(e) => {
            warn!("[sntp] NTP Error: {}", e);
        }
    }

    info!("[sntp] Task Complete!");
}
