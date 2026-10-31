//! Cable DC Loop Resistance ({loop}$) estimation and health modeling.
//!
//! When current flows through a USB-C cable (VBUS and GND conductors, contact pins),
//! a voltage drop occurs according to Ohm's Law: \Delta V = I \cdot R_{loop}.
//! By observing changes in charging voltage and current across dynamic load steps
//! (\Delta V / \Delta I), software can estimate internal cable resistance in milliohms (m\Omega)
//! without needing dedicated inline RF lab instruments.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResistanceRating {
    /// <= 150 mOhm: Excellent conductor and contact integrity (typical high-grade 1-2m cable).
    Nominal,
    /// 151 - 300 mOhm: Moderate resistance (typical of 3m+ cables or thinner gauge conductors).
    Moderate,
    /// 301 - 500 mOhm: Elevated resistance (potential contact oxidation or undersized wire).
    High,
    /// > 500 mOhm: Hazardous resistance (excessive thermal drop, damaged pin or broken strand).
    Critical,
}

impl ResistanceRating {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Nominal => "Nominal (<150 mΩ)",
            Self::Moderate => "Moderate (150-300 mΩ)",
            Self::High => "High (300-500 mΩ)",
            Self::Critical => "Critical (>500 mΩ)",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LoopResistanceReading {
    pub v1_mv: u32,
    pub i1_ma: u32,
    pub v2_mv: u32,
    pub i2_ma: u32,
    pub delta_v_mv: i32,
    pub delta_i_ma: i32,
    pub resistance_mohm: u32,
    pub rating: ResistanceRating,
}

/// Minimum current step required to ensure signal-to-noise ratio is valid (150 mA).
pub const MIN_CURRENT_DELTA_MA: u32 = 150;

/// Estimates loop resistance from a pair of (voltage_mv, current_ma) points.
pub fn calculate_step_resistance(
    v1_mv: u32,
    i1_ma: u32,
    v2_mv: u32,
    i2_ma: u32,
) -> Option<LoopResistanceReading> {
    let delta_i = i2_ma as i64 - i1_ma as i64;
    let delta_v = v1_mv as i64 - v2_mv as i64;

    if delta_i.abs() < MIN_CURRENT_DELTA_MA as i64 {
        return None;
    }

    if (delta_v > 0 && delta_i < 0) || (delta_v < 0 && delta_i > 0) {
        return None;
    }

    let r_mohm_i64 = (delta_v.abs() * 1000) / delta_i.abs();
    if r_mohm_i64 <= 0 || r_mohm_i64 > 5000 {
        return None;
    }

    let resistance_mohm = r_mohm_i64 as u32;
    let rating = if resistance_mohm <= 150 {
        ResistanceRating::Nominal
    } else if resistance_mohm <= 300 {
        ResistanceRating::Moderate
    } else if resistance_mohm <= 500 {
        ResistanceRating::High
    } else {
        ResistanceRating::Critical
    };

    Some(LoopResistanceReading {
        v1_mv,
        i1_ma,
        v2_mv,
        i2_ma,
        delta_v_mv: delta_v as i32,
        delta_i_ma: delta_i as i32,
        resistance_mohm,
        rating,
    })
}

/// Analyzes a time-series of power samples (mv, ma) to detect dynamic load steps
/// and calculate the median estimated loop resistance.
pub fn estimate_from_series(samples: &[(u32, u32)]) -> Option<LoopResistanceReading> {
    if samples.len() < 2 {
        return None;
    }
    let mut valid_readings = Vec::new();
    for window in samples.windows(2) {
        let (v1, i1) = window[0];
        let (v2, i2) = window[1];
        if let Some(r) = calculate_step_resistance(v1, i1, v2, i2) {
            valid_readings.push(r);
        }
    }
    if valid_readings.is_empty() {
        return None;
    }
    valid_readings.sort_by_key(|r| r.resistance_mohm);
    Some(valid_readings[valid_readings.len() / 2].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_cable_resistance() {
        // 20V baseline at 1000mA, drops to 19.88V at 2000mA (+1000mA step, 120mV drop)
        // R = 120mV / 1A = 120 mOhm
        let r = calculate_step_resistance(20000, 1000, 19880, 2000).expect("valid reading");
        assert_eq!(r.resistance_mohm, 120);
        assert_eq!(r.rating, ResistanceRating::Nominal);
    }

    #[test]
    fn high_resistance_detection() {
        // 20V baseline at 500mA, drops to 19.60V at 1500mA (+1000mA step, 400mV drop)
        // R = 400mV / 1A = 400 mOhm
        let r = calculate_step_resistance(20000, 500, 19600, 1500).expect("valid reading");
        assert_eq!(r.resistance_mohm, 400);
        assert_eq!(r.rating, ResistanceRating::High);
    }

    #[test]
    fn rejects_insufficient_current_delta() {
        // Only 50 mA change
        assert!(calculate_step_resistance(20000, 1000, 19990, 1050).is_none());
    }

    #[test]
    fn rejects_violating_ohms_law() {
        // Voltage increases as current increases (indicates active charger regulation or negotiation)
        assert!(calculate_step_resistance(20000, 1000, 20100, 2000).is_none());
    }

    #[test]
    fn series_median_estimation() {
        let samples = vec![
            (20000, 1000),
            (19870, 2000), // 130 mOhm
            (19875, 2000), // noise, no delta
            (19740, 3000), // 135 mOhm
        ];
        let r = estimate_from_series(&samples).expect("median reading");
        assert!(r.resistance_mohm >= 130 && r.resistance_mohm <= 135);
        assert_eq!(r.rating, ResistanceRating::Nominal);
    }
}
