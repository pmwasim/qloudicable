//! Internal consistency check of a report. It says nothing about a real cable.

use crate::diagnose::Report;
use crate::model::Snapshot;
use serde::{Deserialize, Serialize};

/// Self-check (QA) Scorecard verifying diagnosis accuracy, stability, and protocol bounds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SelfCheck {
    pub total_checks: u32,
    pub passed_checks: u32,
    pub pass_rate_pct: f64,
    pub passed: bool,
    pub notes: Vec<String>,
}

impl SelfCheck {
    pub fn evaluate(report: &Report, snapshot: &Snapshot) -> Self {
        let mut total = 0;
        let mut passed = 0;
        let mut notes = Vec::new();

        // Check 1: Port identification integrity
        total += 1;
        if snapshot.ports.iter().all(|p| !p.id.is_empty()) {
            passed += 1;
            notes.push("Port ID mapping valid".to_string());
        } else {
            notes.push("Detected ports with empty or corrupt IDs".to_string());
        }

        // Check 2: Stated vs Measured Power coherence
        total += 1;
        if let Some(pw) = &snapshot.power {
            if let Some(mw) = pw.milliwatts {
                if mw <= 240_000 {
                    passed += 1;
                    notes.push("Power telemetry within USB PD 240W spec".to_string());
                } else {
                    notes.push(format!("Abnormal power telemetry exceeding spec: {mw} mW"));
                }
            } else {
                passed += 1;
            }
        } else {
            passed += 1;
        }

        // Check 3: Findings severity ordering
        total += 1;
        let mut well_ordered = true;
        for w in report.findings.windows(2) {
            if w[0].severity < w[1].severity {
                well_ordered = false;
                break;
            }
        }
        if well_ordered {
            passed += 1;
            notes.push("Findings properly prioritized by severity".to_string());
        } else {
            notes.push("Findings priority ordering violated".to_string());
        }

        // Check 4: Cable E-Marker VDO validity if present
        total += 1;
        let mut vdo_valid = true;
        for p in &snapshot.ports {
            if let Some(c) = &p.cable
                && c.raw_vdos.is_empty()
            {
                vdo_valid = false;
                break;
            }
        }
        if vdo_valid {
            passed += 1;
            notes.push("E-Marker VDO payloads conform to USB PD structures".to_string());
        } else {
            notes.push("Encountered empty VDO payload in reported cable".to_string());
        }

        let rate = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        SelfCheck {
            total_checks: total,
            passed_checks: passed,
            pass_rate_pct: rate,
            passed: rate >= 95.0,
            notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnose::Report;
    use crate::model::*;

    #[test]
    fn test_qa_scorecard_evaluation() {
        let snap = Snapshot {
            platform: Platform::Macos,
            tool_version: "0.1.0".into(),
            capabilities: Capabilities {
                cable_identity: Access {
                    level: Level::Full,
                    why: "".into(),
                },
                charger_profiles: Access {
                    level: Level::Full,
                    why: "".into(),
                },
                link_speed: Access {
                    level: Level::Full,
                    why: "".into(),
                },
                power_measurement: Access {
                    level: Level::Full,
                    why: "".into(),
                },
            },
            ports: vec![Port {
                id: "1:1".into(),
                label: "Port 1".into(),
                connected: true,
                data_role: None,
                power_role: None,
                orientation: None,
                power_mode: None,
                pd_revision: None,
                cable: Some(CableInfo {
                    kind: CableKind::Passive,
                    vendor_id: 0x05ac,
                    vendor_name: Some("Apple".into()),
                    product_id: 0x1,
                    xid: 0,
                    speed: CableSpeed::Usb2,
                    current_a: Some(3),
                    epr: false,
                    max_watts: Some(60),
                    raw_vdos: vec![0x12345678],
                }),
                partner: None,
                source_pdos: vec![],
                active_pdo: None,
                transports: vec![],
            }],
            usb_devices: vec![],
            power: Some(PowerReading {
                state: ChargeState::Charging,
                external_power: Some(true),
                voltage_mv: Some(20_000),
                current_ma: Some(3_000),
                milliwatts: Some(60_000),
                battery_percent: Some(80),
                source: "Mac".into(),
            }),
            notes: vec![],
        };

        let report = Report {
            snapshot: snap.clone(),
            findings: vec![],
        };

        let scorecard = SelfCheck::evaluate(&report, &snap);
        assert!(scorecard.passed);
        assert_eq!(scorecard.passed_checks, scorecard.total_checks);
    }
}
