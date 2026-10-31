//! Statistics for the two active tests. The OS-facing sampling lives in the backend.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    Steady,
    Fluctuating,
    Dropouts,
    NoData,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChargeTest {
    pub samples: usize,
    pub avg_w: f64,
    pub min_w: f64,
    pub max_w: f64,
    pub stability: Stability,
    pub summary: String,
}

/// `watts` holds one reading per sample. A reading at zero while others are
/// positive is a dropout: power actually stopped during the run.
pub fn charge_test(watts: &[f64]) -> ChargeTest {
    if watts.is_empty() {
        return ChargeTest {
            samples: 0,
            avg_w: 0.0,
            min_w: 0.0,
            max_w: 0.0,
            stability: Stability::NoData,
            summary: "This device gave no power readings, so the test could not run.".into(),
        };
    }
    let avg = watts.iter().sum::<f64>() / watts.len() as f64;
    let min = watts.iter().copied().fold(f64::INFINITY, f64::min);
    let max = watts.iter().copied().fold(0.0, f64::max);
    let sd = (watts.iter().map(|w| (w - avg).powi(2)).sum::<f64>() / watts.len() as f64).sqrt();
    let dropout = max > 1.0 && min < 0.5;
    let stability = if dropout {
        Stability::Dropouts
    } else if avg > 0.0 && sd / avg > 0.25 {
        Stability::Fluctuating
    } else {
        Stability::Steady
    };
    let summary = match stability {
        Stability::Dropouts => format!(
            "Power dropped to zero during the test (peak {max:.1} W). Try another cable: a worn cable or loose plug often does this."
        ),
        Stability::Fluctuating => format!(
            "Power swung between {min:.1} W and {max:.1} W. Normal if the device is busy; if it is idle, suspect the cable or charger."
        ),
        _ if max < 0.5 => "No charging power during the test. Check the charger is plugged in and switched on.".into(),
        _ => format!("Steady at about {avg:.1} W with no dropouts."),
    };
    ChargeTest { samples: watts.len(), avg_w: avg, min_w: min, max_w: max, stability, summary }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DataTest {
    pub bytes: u64,
    pub seconds: f64,
    pub write_mbps: f64,
    pub link_mbps: Option<u32>,
    pub summary: String,
}

pub fn data_test(bytes: u64, seconds: f64, link_mbps: Option<u32>) -> DataTest {
    let write_mbps = if seconds > 0.0 { bytes as f64 * 8.0 / 1e6 / seconds } else { 0.0 };
    let summary = match link_mbps {
        // Protocol overhead takes a sizeable share of USB 2 and a smaller one of
        // USB 3, and most drives are slower than their link, so only a result
        // far below the link deserves a remark.
        Some(link) if write_mbps < link as f64 * 0.1 => format!(
            "Wrote at {write_mbps:.0} Mbps on a {link} Mbps link. The drive itself is the likely limit, not the cable."
        ),
        Some(link) => format!("Wrote at {write_mbps:.0} Mbps on a {link} Mbps link."),
        None => format!("Wrote at {write_mbps:.0} Mbps. This system did not report the link speed."),
    };
    DataTest { bytes, seconds, write_mbps, link_mbps, summary }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_runs() {
        assert_eq!(charge_test(&[]).stability, Stability::NoData);
        assert_eq!(charge_test(&[30.0, 30.5, 29.8]).stability, Stability::Steady);
        assert_eq!(charge_test(&[10.0, 40.0, 12.0, 38.0]).stability, Stability::Fluctuating);
        assert_eq!(charge_test(&[30.0, 0.0, 30.0]).stability, Stability::Dropouts);
        // Never charging is steady at nothing, not a dropout.
        assert_eq!(charge_test(&[0.0, 0.0]).stability, Stability::Steady);
        assert!(charge_test(&[0.0, 0.0]).summary.starts_with("No charging power"));
    }

    #[test]
    fn battery_drain_is_not_charging_power() {
        use crate::{ChargeState, PowerReading};
        let drain = PowerReading { state: ChargeState::Discharging, external_power: Some(false), milliwatts: Some(4500), ..Default::default() };
        assert_eq!(drain.charging_milliwatts(), Some(0));
        let charging = PowerReading { state: ChargeState::Charging, external_power: Some(true), milliwatts: Some(4500), ..Default::default() };
        assert_eq!(charging.charging_milliwatts(), Some(4500));
        // Unplugging mid-run must not read as steady.
        let run = [charging.charging_milliwatts(), drain.charging_milliwatts()].map(|mw| mw.unwrap() as f64 / 1000.0);
        assert_eq!(charge_test(&run).stability, Stability::Dropouts);
    }

    #[test]
    fn data_rate_maths() {
        let t = data_test(125_000_000, 1.0, Some(5000));
        assert_eq!(t.write_mbps, 1000.0);
        assert!(data_test(1_000_000, 1.0, Some(5000)).summary.contains("drive"));
        assert!(data_test(1, 0.0, None).summary.contains("did not report"));
    }
}
