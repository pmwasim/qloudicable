//! Platform-free heart of QloudiCable: the data model, USB Power Delivery decoding,
//! the diagnostic engine and the test statistics. Nothing here touches the OS, so
//! all of it is testable on any machine with fixture data.
pub mod bench;
pub mod diagnose;
pub mod model;
pub mod pd;
pub mod render;
pub mod resistance;
pub mod stability;
pub mod test_matrix;
pub mod vendors;
pub mod virtual_engine;

pub use diagnose::{Report, diagnose};
pub use model::*;
