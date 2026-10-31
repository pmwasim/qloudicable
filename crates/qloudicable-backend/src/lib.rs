//! Reads the operating system. Each platform module fills the same `Snapshot`, and
//! says through `Capabilities` what that OS keeps hidden.
pub mod android;
// Test fixtures use file names with colons, as sysfs does, which Windows refuses.
#[cfg(any(target_os = "linux", target_os = "android", all(test, unix)))]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod macos_parse;
pub mod tests_active;
#[cfg(windows)]
pub mod windows;

use qloudicable_core::Snapshot;

/// Desktop entry point. Android goes through `android::snapshot`, because its data
/// arrives from Kotlin.
pub fn collect() -> Snapshot {
    #[cfg(target_os = "linux")]
    let mut s = linux::collect(std::path::Path::new("/"));
    #[cfg(target_os = "macos")]
    let mut s = macos::collect();
    #[cfg(windows)]
    let mut s = windows::collect();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    let mut s = Snapshot::default();
    s.tool_version = qloudicable_core::version_label();
    s
}

/// USB's device-speed classes for a bcdUSB value. A USB 3 device on a USB 2 path
/// can present a 2.x descriptor, so a low number never proves low capability.
pub fn capable_from_version(version: &str) -> Option<u32> {
    let major: u32 = version.trim().split('.').next()?.parse().ok()?;
    (major >= 3).then_some(5_000)
}
