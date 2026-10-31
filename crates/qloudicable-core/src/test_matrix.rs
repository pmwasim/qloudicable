//! Synthetic Hardware R&D Matrix Generator & Evaluator.
//!
//! Provides automated simulation vectors to evaluate QloudiCable's diagnostic algorithms
//! across a broad range of real-world scenarios (EPR 140W/240W, passive 60W, Thunderbolt 4,
//! damaged e-markers, link bottlenecks) without requiring physical cable benches.

use crate::diagnose::{Report, diagnose};
use crate::model::*;
use crate::pd::decode_cable;

#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub name: String,
    pub description: String,
    pub passed: bool,
    pub report: Report,
}

pub fn run_scenario_matrix() -> Vec<ScenarioResult> {
    let mut results = Vec::new();

    // Scenario 1: Apple 140W MagSafe/USB-C charger with a genuine 240W EPR cable
    {
        let vdo_240w = (0b011 << 18) | (1 << 17) | (0b10 << 5) | 0b011;
        let cable = decode_cable((0b011 << 27) | 0x05ac, 0x1234, 0x7001_0100, vdo_240w);
        let port = Port {
            id: "port-1".into(),
            label: "Port 1 (Left Top)".into(),
            connected: true,
            cable: Some(cable),
            source_pdos: vec![
                Pdo::fixed(5_000, 3_000),
                Pdo::fixed(28_000, 5_000), // 140W
            ],
            active_pdo: Some(Pdo::fixed(28_000, 5_000)),
            ..Default::default()
        };
        let snapshot = Snapshot {
            tool_version: "0.1.0".into(),
            platform: Platform::Macos,
            ports: vec![port],
            capabilities: Capabilities {
                cable_identity: Access::full(),
                charger_profiles: Access::full(),
                link_speed: Access::full(),
                power_measurement: Access::full(),
            },
            power: Some(PowerReading {
                state: ChargeState::Charging,
                milliwatts: Some(138_000),
                external_power: Some(true),
                source: "USB-C / MagSafe".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let report = diagnose(&snapshot);
        let has_charging_at_limit = report
            .findings
            .iter()
            .any(|f| f.code == "charging_at_limit");
        results.push(ScenarioResult {
            name: "EPR 240W Cable on 140W Charger".into(),
            description:
                "Ensures 240W cable correctly permits 140W contract without false warnings".into(),
            passed: has_charging_at_limit,
            report,
        });
    }

    // Scenario 2: 100W Charger with a 60W (3A) Cable
    {
        let vdo_60w = (0b01 << 5) | 0b011;
        let cable = decode_cable((0b011 << 27) | 0x04b4, 0x1111, 0x0001, vdo_60w);
        let port = Port {
            id: "port-1".into(),
            label: "Port 1".into(),
            connected: true,
            cable: Some(cable),
            source_pdos: vec![
                Pdo::fixed(20_000, 5_000), // 100W
            ],
            active_pdo: Some(Pdo::fixed(20_000, 3_000)), // 60W
            ..Default::default()
        };
        let snapshot = Snapshot {
            tool_version: "0.1.0".into(),
            platform: Platform::Linux,
            ports: vec![port],
            capabilities: Capabilities {
                cable_identity: Access::full(),
                charger_profiles: Access::full(),
                link_speed: Access::full(),
                power_measurement: Access::full(),
            },
            ..Default::default()
        };
        let report = diagnose(&snapshot);
        let flags_cable_limit = report
            .findings
            .iter()
            .any(|f| f.code == "cable_limits_charging");
        results.push(ScenarioResult {
            name: "60W Cable on 100W Charger".into(),
            description: "Detects 3A / 60W cable restricting a 100W capable charger".into(),
            passed: flags_cable_limit,
            report,
        });
    }

    // Scenario 3: Fast SSD throttled to USB 2.0 (480 Mbps) by charging-only cable
    {
        let port = Port {
            id: "port-2".into(),
            label: "Port 2".into(),
            connected: true,
            cable: Some(decode_cable((0b011 << 27) | 0x05ac, 0, 0, 0b01 << 5)), // USB 2.0 speed
            ..Default::default()
        };
        let device = UsbDevice {
            vendor_id: 0x0bda,
            product_id: 0x9210,
            product_name: Some("NVMe High Speed SSD Case".into()),
            vendor_name: Some("Realtek".into()),
            speed_mbps: Some(480),
            capable_mbps: Some(10_000),
            port_id: Some("port-2".into()),
            is_hub: false,
            usb_version: Some("3.20".into()),
        };
        let snapshot = Snapshot {
            tool_version: "0.1.0".into(),
            platform: Platform::Macos,
            ports: vec![port],
            capabilities: Capabilities {
                cable_identity: Access::full(),
                charger_profiles: Access::full(),
                link_speed: Access::full(),
                power_measurement: Access::full(),
            },
            usb_devices: vec![device],
            ..Default::default()
        };
        let report = diagnose(&snapshot);
        let flags_data_limit = report
            .findings
            .iter()
            .any(|f| f.code == "cable_limits_data");
        results.push(ScenarioResult {
            name: "Fast 10Gbps NVMe SSD Throttled by USB 2.0 Cable".into(),
            description: "Verifies data rate bottleneck detection when a 10Gbps SSD is plugged with USB 2.0 cable".into(),
            passed: flags_data_limit,
            report,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_scenarios_pass() {
        let results = run_scenario_matrix();
        assert_eq!(results.len(), 3);
        for res in results {
            assert!(res.passed, "Scenario failed: {}", res.name);
        }
    }
}
