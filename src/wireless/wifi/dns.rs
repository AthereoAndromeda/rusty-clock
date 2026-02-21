use core::net::IpAddr;
use embassy_time::{Duration, WithTimeout as _};

#[derive(Debug, defmt::Format, thiserror::Error)]
pub(crate) enum DnsError {
    #[error("DNS Request Timed Out")]
    /// Same as [`embassy_time::TimeoutError`].
    Timeout,

    #[error("DNS Socket Error: {0:?}")]
    /// Errors from [`embassy_net::dns::Error`] and DNS Socket.
    DnsSocketError(embassy_net::dns::Error),

    #[error("No Addresses Returned")]
    /// DNS resolution returns empty vec.
    NoAddrs,
}

// We have to impl manually since `embassy` errors don't implement [`core::error::Error`].
impl From<embassy_net::dns::Error> for DnsError {
    fn from(value: embassy_net::dns::Error) -> Self {
        DnsError::DnsSocketError(value)
    }
}

impl From<embassy_time::TimeoutError> for DnsError {
    fn from(_value: embassy_time::TimeoutError) -> Self {
        DnsError::Timeout
    }
}

/// Fallibly send a DNS request and return addresses.
///
/// WARN: Will not retry if a DNS error occurs. Up to the caller to retry.
pub(crate) async fn resolve(
    server_name: &str,
    net_stack: embassy_net::Stack<'_>,
) -> Result<IpAddr, DnsError> {
    let ntp_addrs_response = net_stack
        .dns_query(server_name, smoltcp::wire::DnsQueryType::A)
        .with_timeout(Duration::from_secs(180))
        .await
        .map_err(DnsError::from)?; // Timeout Error

    let ntp_addrs = match ntp_addrs_response {
        Err(err) => return Err(err.into()),
        Ok(addr) if addr.is_empty() => return Err(DnsError::NoAddrs),
        Ok(addr) => addr,
    };

    #[expect(clippy::indexing_slicing, reason = "Guaranteed to be non-empty")]
    // Converts `smoltcp Address` to `IpAddr`
    let addr = ntp_addrs[0].into();
    Ok(addr)
}
