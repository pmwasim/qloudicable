//! Platform-free heart of QloudiCable: the data model, USB Power Delivery decoding,
//! the diagnostic engine and the test statistics. Nothing here touches the OS, so
//! all of it is testable on any machine with fixture data.
pub mod bench;
pub mod diagnose;
pub mod model;
pub mod pd;
pub mod render;
pub mod resistance;
pub mod selfcheck;
pub mod stability;
pub mod test_matrix;
pub mod vendors;
pub mod virtual_engine;

pub use diagnose::{Report, diagnose};
pub use model::*;

/// Release channel set at build time by CI: "beta" or "stable". Local builds are "dev".
pub const CHANNEL: &str = match option_env!("QLOUDICABLE_CHANNEL") {
    Some(c) => c,
    None => "dev",
};
/// The version shown to users, for example "0.2.0" or "0.2.0 (beta)".
pub fn version_label() -> String {
    let v = env!("CARGO_PKG_VERSION");
    if CHANNEL == "stable" {
        v.to_string()
    } else {
        format!("{v} ({CHANNEL})")
    }
}
