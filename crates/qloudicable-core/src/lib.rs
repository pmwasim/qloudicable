//! Platform-free heart of QloudiCable: the data model, USB Power Delivery decoding,
//! the diagnostic engine and the test statistics. Nothing here touches the OS, so
//! all of it is testable on any machine with fixture data.
pub mod bench;
pub mod diagnose;
pub mod model;
pub mod pd;
pub mod render;
pub mod vendors;

pub use diagnose::{Report, diagnose};
pub use model::*;
