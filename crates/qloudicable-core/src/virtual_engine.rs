//! Virtual Cable Testing Engine.
//!
//! Provides an offline, non-metered simulation engine that models electrical,
//! PD protocol, data signaling, and thermal characteristics of USB-C cables.
//! Enables developers and automated workers to stress-test diagnostic logic
//! and simulate physical cables without external lab instruments.

use crate::diagnose::{Report, diagnose};
use crate::model::*;
use crate::pd::decode_cable;
use crate::resistance::{LoopResistanceReading, ResistanceRating};
use crate::stability::{EventType, PortEvent, StabilityVerdict, analyze_stability};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(non_camel_case_types)]
pub enum VirtualCableProfile {
    /// Basic USB 2.0 charging cable with 3A e-marker or passive wire (60W max, 480 Mbps).
    Usb2Charging60W,
    /// High-wattage EPR charging cable (USB 2.0 data, 240W 5A EPR, e.g. Apple 2m charge cable).
    AppleWovenEpr240W,
    /// USB 3.2 Gen 2 full-featured cable (10 Gbps, 5A 100W, e-marked).
    Usb3Gen2_10Gbps_100W,
    /// USB4 Gen 3x2 certified cable (40 Gbps, 5A 240W EPR, e-marked).
    Usb4Gen3x2_40Gbps_240W,
    /// Thunderbolt 4 cable (40 Gbps, 100W or 240W, active/passive).
    Thunderbolt4_40Gbps,
    /// Counterfeit or damaged cable: zero vendor ID, mismatched VDO, high resistance, intermittent dropouts.
    CounterfeitOrDamaged,
    /// Cheap unmarked cable: no e-marker chip, unknown wire gauge, spec-limited to 60W.
    UnmarkedPassive,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VirtualTarget {
    Charger { max_watts: u32 },
    HighSpeedStorage { capable_mbps: u32 },
    HighResolutionDisplay { bandwidth_gbps: u32 },
    LegacyPeripheral,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VirtualTestReport {
    pub profile: VirtualCableProfile,
    pub target: VirtualTarget,
    pub negotiated_watts: Option<u32>,
    pub negotiated_mbps: Option<u32>,
    pub cable_is_power_bottleneck: bool,
    pub cable_is_data_bottleneck: bool,
    pub estimated_loop_resistance: LoopResistanceReading,
    pub thermal_dissipation_watts: f64,
    pub thermal_risk: bool,
    pub stability_score: u8,
    pub stability_verdict: StabilityVerdict,
    pub report: Report,
}

pub struct VirtualTester;

impl VirtualTester {
    pub fn evaluate(profile: &VirtualCableProfile, target: &VirtualTarget) -> VirtualTestReport {
        // 1. Synthesize cable electrical and protocol facts
        let (cable_info, resistance_mohm, dropouts) = match profile {
            VirtualCableProfile::Usb2Charging60W => {
                let vdo = 0b01 << 5; // 3A, non-EPR, USB 2.0
                (
                    Some(decode_cable((0b011 << 27) | 0x05ac, 0x1111, 0x0001, vdo)),
                    110,
                    0,
                )
            }
            VirtualCableProfile::AppleWovenEpr240W => {
                let vdo = (0b011 << 18) | (1 << 17) | (0b10 << 5); // 5A, 240W EPR, USB 2.0
                (
                    Some(decode_cable((0b011 << 27) | 0x05ac, 0x1111, 0x0002, vdo)),
                    95,
                    0,
                )
            }
            VirtualCableProfile::Usb3Gen2_10Gbps_100W => {
                let vdo = (0b10 << 5) | 0b010; // 5A 100W, USB 3.2 Gen 2
                (
                    Some(decode_cable((0b011 << 27) | 0x04b4, 0x2222, 0x0003, vdo)),
                    125,
                    0,
                )
            }
            VirtualCableProfile::Usb4Gen3x2_40Gbps_240W => {
                let vdo = (0b011 << 18) | (1 << 17) | (0b10 << 5) | 0b011; // 5A 240W EPR, 40 Gbps
                (
                    Some(decode_cable((0b011 << 27) | 0x04b4, 0x3333, 0x0004, vdo)),
                    105,
                    0,
                )
            }
            VirtualCableProfile::Thunderbolt4_40Gbps => {
                let vdo = (0b011 << 18) | (1 << 17) | (0b10 << 5) | 0b011;
                (
                    Some(decode_cable((0b011 << 27) | 0x8087, 0x4444, 0x0005, vdo)),
                    100,
                    0,
                )
            }
            VirtualCableProfile::CounterfeitOrDamaged => {
                // Zero vendor ID, corrupt VDO, high resistance, contact dropouts
                let vdo = (0b11 << 5) | 0b111;
                (Some(decode_cable(0b011 << 27, 0, 0, vdo)), 480, 3)
            }
            VirtualCableProfile::UnmarkedPassive => (None, 160, 0),
        };

        let cable_max_w = cable_info.as_ref().and_then(|c| c.max_watts).unwrap_or(60);
        let cable_speed_mbps = cable_info
            .as_ref()
            .and_then(|c| c.speed.mbps())
            .unwrap_or(480);

        // 2. Synthesize Target & Host Interactions
        let mut source_pdos = Vec::new();
        let mut usb_devices = Vec::new();
        let mut negotiated_watts = None;
        let mut negotiated_mbps = None;
        let mut power_reading = None;

        match target {
            VirtualTarget::Charger { max_watts } => {
                source_pdos.push(Pdo::fixed(5000, 3000));
                source_pdos.push(Pdo::fixed(20000, (*max_watts * 1000) / 20000));
                let allowed_w = (*max_watts).min(cable_max_w);
                negotiated_watts = Some(allowed_w);
                let current_a = (allowed_w as f64 / 20.0).max(1.0);
                power_reading = Some(PowerReading {
                    state: ChargeState::Charging,
                    external_power: Some(true),
                    voltage_mv: Some(20000),
                    current_ma: Some((current_a * 1000.0) as u32),
                    milliwatts: Some(allowed_w * 1000),
                    battery_percent: Some(50),
                    source: "USB-C Power Adapter".into(),
                });
            }
            VirtualTarget::HighSpeedStorage { capable_mbps } => {
                let link_speed = (*capable_mbps).min(cable_speed_mbps);
                negotiated_mbps = Some(link_speed);
                usb_devices.push(UsbDevice {
                    vendor_id: 0x1234,
                    product_id: 0x5678,
                    product_name: Some("SuperSpeed+ NVMe Enclosure".into()),
                    vendor_name: Some("FastDisk".into()),
                    speed_mbps: Some(link_speed),
                    capable_mbps: Some(*capable_mbps),
                    port_id: Some("sim-port-1".into()),
                    is_hub: false,
                    usb_version: Some("3.20".into()),
                });
            }
            VirtualTarget::HighResolutionDisplay { bandwidth_gbps } => {
                let capable_mbps = bandwidth_gbps * 1000;
                let link_speed = capable_mbps.min(cable_speed_mbps);
                negotiated_mbps = Some(link_speed);
                usb_devices.push(UsbDevice {
                    vendor_id: 0x04b4,
                    product_id: 0x9999,
                    product_name: Some("UltraHD 4K/8K Display Engine".into()),
                    vendor_name: Some("VisionCorp".into()),
                    speed_mbps: Some(link_speed),
                    capable_mbps: Some(capable_mbps),
                    port_id: Some("sim-port-1".into()),
                    is_hub: false,
                    usb_version: Some("4.00".into()),
                });
            }
            VirtualTarget::LegacyPeripheral => {
                negotiated_mbps = Some(480);
                usb_devices.push(UsbDevice {
                    vendor_id: 0x046d,
                    product_id: 0xc52b,
                    product_name: Some("USB Receiver".into()),
                    vendor_name: Some("Logitech".into()),
                    speed_mbps: Some(480),
                    capable_mbps: Some(480),
                    port_id: Some("sim-port-1".into()),
                    is_hub: false,
                    usb_version: Some("2.00".into()),
                });
            }
        }

        let port = Port {
            id: "sim-port-1".into(),
            label: "Virtual Port 1".into(),
            connected: true,
            cable: cable_info.clone(),
            source_pdos: source_pdos.clone(),
            active_pdo: negotiated_watts.map(|w| Pdo::fixed(20000, (w * 1000) / 20000)),
            ..Default::default()
        };

        let snapshot = Snapshot {
            tool_version: "0.1.0".into(),
            platform: Platform::Linux,
            capabilities: Capabilities {
                cable_identity: Access::full(),
                charger_profiles: Access::full(),
                link_speed: Access::full(),
                power_measurement: Access::full(),
            },
            ports: vec![port],
            usb_devices,
            power: power_reading,
            ..Default::default()
        };

        let report = diagnose(&snapshot);

        // 3. Evaluate Bottlenecks
        let cable_is_power_bottleneck = match target {
            VirtualTarget::Charger { max_watts } => cable_max_w < *max_watts,
            _ => false,
        };

        let cable_is_data_bottleneck = match target {
            VirtualTarget::HighSpeedStorage { capable_mbps } => cable_speed_mbps < *capable_mbps,
            VirtualTarget::HighResolutionDisplay { bandwidth_gbps } => {
                cable_speed_mbps < (*bandwidth_gbps * 1000)
            }
            _ => false,
        };

        // 4. Electrical Loop Resistance & Thermal Modeling: P_loss = I^2 * R
        let current_amps = negotiated_watts
            .map(|w| (w as f64 / if w > 100 { 28.0 } else { 20.0 }).clamp(0.5, 5.0))
            .unwrap_or(0.5);
        let r_ohms = resistance_mohm as f64 / 1000.0;
        let thermal_dissipation_watts = current_amps * current_amps * r_ohms;
        let thermal_risk = thermal_dissipation_watts > 3.0;

        let resistance_rating = if resistance_mohm <= 150 {
            ResistanceRating::Nominal
        } else if resistance_mohm <= 300 {
            ResistanceRating::Moderate
        } else if resistance_mohm <= 500 {
            ResistanceRating::High
        } else {
            ResistanceRating::Critical
        };

        let loop_reading = LoopResistanceReading {
            v1_mv: 20000,
            i1_ma: (current_amps * 1000.0) as u32,
            v2_mv: (20000.0 - (current_amps * r_ohms * 1000.0)) as u32,
            i2_ma: (current_amps * 1000.0) as u32,
            delta_v_mv: (current_amps * r_ohms * 1000.0) as i32,
            delta_i_ma: 0,
            resistance_mohm,
            rating: resistance_rating,
        };

        // 5. Stability & Contact Resilience Analysis
        let mut events = vec![PortEvent {
            timestamp_epoch_s: 1730000000,
            port_id: "sim-port-1".into(),
            event_type: EventType::Connect,
            detail: "Initial plug-in".into(),
        }];
        for i in 0..dropouts {
            events.push(PortEvent {
                timestamp_epoch_s: 1730000005 + (i * 2) as u64,
                port_id: "sim-port-1".into(),
                event_type: EventType::Disconnect,
                detail: "Intermittent micro-dropout".into(),
            });
            events.push(PortEvent {
                timestamp_epoch_s: 1730000006 + (i * 2) as u64,
                port_id: "sim-port-1".into(),
                event_type: EventType::Connect,
                detail: "Re-established".into(),
            });
        }
        let stability = analyze_stability(&events);

        VirtualTestReport {
            profile: profile.clone(),
            target: target.clone(),
            negotiated_watts,
            negotiated_mbps,
            cable_is_power_bottleneck,
            cable_is_data_bottleneck,
            estimated_loop_resistance: loop_reading,
            thermal_dissipation_watts,
            thermal_risk,
            stability_score: stability.score,
            stability_verdict: stability.verdict,
            report,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usb4_cable_on_fast_charger_and_ssd() {
        let profile = VirtualCableProfile::Usb4Gen3x2_40Gbps_240W;
        let charger = VirtualTarget::Charger { max_watts: 140 };
        let res = VirtualTester::evaluate(&profile, &charger);
        assert_eq!(res.negotiated_watts, Some(140));
        assert!(!res.cable_is_power_bottleneck);
        assert!(!res.thermal_risk);
        assert_eq!(res.stability_score, 100);

        let ssd = VirtualTarget::HighSpeedStorage {
            capable_mbps: 10_000,
        };
        let res_ssd = VirtualTester::evaluate(&profile, &ssd);
        assert_eq!(res_ssd.negotiated_mbps, Some(10_000));
        assert!(!res_ssd.cable_is_data_bottleneck);
    }

    #[test]
    fn test_counterfeit_cable_flags_risk() {
        let profile = VirtualCableProfile::CounterfeitOrDamaged;
        let charger = VirtualTarget::Charger { max_watts: 100 };
        let res = VirtualTester::evaluate(&profile, &charger);
        // Resistance 480 mOhm at 3A produces high thermal dissipation (>3.5W)
        assert!(res.thermal_risk);
        assert!(res.stability_score < 70);
        assert_eq!(res.stability_verdict, StabilityVerdict::Jittery);
        assert!(
            res.report
                .findings
                .iter()
                .any(|f| f.code == "unusual_emarker")
        );
    }

    #[test]
    fn test_charging_cable_bottlenecks_ssd() {
        let profile = VirtualCableProfile::AppleWovenEpr240W; // 240W power, but only USB 2.0 (480 Mbps) data
        let ssd = VirtualTarget::HighSpeedStorage {
            capable_mbps: 10_000,
        };
        let res = VirtualTester::evaluate(&profile, &ssd);
        assert_eq!(res.negotiated_mbps, Some(480));
        assert!(res.cable_is_data_bottleneck);
        assert!(
            res.report
                .findings
                .iter()
                .any(|f| f.code == "cable_limits_data")
        );
    }
}
