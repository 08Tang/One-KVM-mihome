//! Video device discovery, capability probing, and platform adapters.

#[cfg(unix)]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use linux::{
    enumerate_devices, find_best_device, select_recovery_device, VideoDevice, VideoDeviceInfo,
    VideoDeviceRecoveryHint,
};
#[cfg(windows)]
pub use windows::*;

#[cfg(unix)]
pub mod bridge;
#[cfg(windows)]
#[path = "disabled_bridge.rs"]
pub mod bridge;

pub(crate) fn is_rk_hdmirx_driver(driver: &str, card: &str) -> bool {
    [driver, card].iter().any(|name| {
        name.eq_ignore_ascii_case("rk_hdmirx") || name.eq_ignore_ascii_case("snps_hdmirx")
    })
}

pub(crate) fn is_rk_hdmirx_device(device: &VideoDeviceInfo) -> bool {
    is_rk_hdmirx_driver(&device.driver, &device.card)
}

pub(crate) fn is_rkcif_driver(driver: &str) -> bool {
    driver.eq_ignore_ascii_case("rkcif")
}

/// Unified check for CSI/HDMI bridge devices (rk_hdmirx, rkcif, etc.)
/// that require special enumeration and format-selection logic.
pub(crate) fn is_csi_hdmi_bridge(device: &VideoDeviceInfo) -> bool {
    is_rk_hdmirx_device(device) || is_rkcif_driver(&device.driver)
}

#[cfg(test)]
mod tests {
    use super::is_rk_hdmirx_driver;

    #[test]
    fn recognizes_vendor_and_upstream_native_hdmirx_names() {
        assert!(is_rk_hdmirx_driver("rk_hdmirx", "rk_hdmirx"));
        assert!(is_rk_hdmirx_driver("snps_hdmirx", "Synopsys HDMI RX"));
        assert!(is_rk_hdmirx_driver("other", "SNPS_HDMIRX"));
        assert!(!is_rk_hdmirx_driver("rkcif", "stream_cif_mipi_id0"));
    }
}

#[cfg(unix)]
pub(crate) use linux::parse_bridge_kind;
