use core::net::IpAddr;

use defmt::warn;
use embassy_time::{Duration, WithTimeout as _};

#[derive(Debug, defmt::Format, thiserror::Error)]
pub(crate) enum DnsError {
    #[error("DNS Request Timed Out")]
    /// Embassy timeout.
    Timeout,

    #[error("DNS Socket Error: {0:?}")]
    /// Errors from [`embassy_net::dns::Error`] and DNS Socket.
    DnsSocketError(embassy_net::dns::Error),

    #[error("No Addresses Returned")]
    /// DNS resolution returns empty vec.
    NoAddrs,
}

impl From<embassy_net::dns::Error> for DnsError {
    fn from(value: embassy_net::dns::Error) -> Self {
        DnsError::DnsSocketError(value)
    }
}

/// Fallibly send a DNS request and return addresses.
///
/// WARN: Will not retry if a DNS error occurs. Up to the caller to retry
pub(crate) async fn resolve(
    server_name: &str,
    net_stack: embassy_net::Stack<'_>,
) -> Result<heapless::Vec<IpAddr, { smoltcp::config::DNS_MAX_RESULT_COUNT }>, DnsError> {
    let ntp_addrs_future = net_stack
        .dns_query(server_name, smoltcp::wire::DnsQueryType::A)
        .with_timeout(Duration::from_secs(180))
        .await;

    let Ok(ntp_addrs_response) = ntp_addrs_future else {
        warn!("[sntp] DNS Request Timeout!");
        return Err(DnsError::Timeout);
    };

    let ntp_addrs = match ntp_addrs_response {
        Ok(addr) if addr.is_empty() => return Err(DnsError::NoAddrs),
        Ok(addr) => addr,
        Err(e) => {
            warn!("[sntp] DNS Request Failed: {}", e);
            return Err(DnsError::DnsSocketError(e));
        }
    };

    // if ntp_addrs.is_empty() {
    //     warn!("[sntp] DNS Resolution Failed: No addrs received\nFalling back to stored RTC time");
    //     return Err(DnsError::NoAddrs);
    // }

    // Converts `smoltcp Address` to `IpAddr`
    Ok(ntp_addrs.into_iter().map(Into::into).collect())
}
