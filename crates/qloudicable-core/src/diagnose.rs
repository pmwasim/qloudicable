//! Turns a snapshot into plain-language findings. Each finding says how we know
//! it, so a user can tell a fact the hardware reported from our own inference.
use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Good,
    Info,
    Warn,
    Problem,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Evidence {
    /// The hardware stated this value.
    Reported,
    /// We timed or metered this.
    Measured,
    /// We worked this out from other facts or from the USB specification.
    Inferred,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Finding {
    pub port: Option<String>,
    pub severity: Severity,
    pub code: String,
    pub title: String,
    pub detail: String,
    pub evidence: Evidence,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Report {
    pub snapshot: Snapshot,
    pub findings: Vec<Finding>,
}

fn finding(
    port: Option<&str>,
    severity: Severity,
    code: &str,
    title: String,
    detail: String,
    evidence: Evidence,
) -> Finding {
    Finding {
        port: port.map(str::to_string),
        severity,
        code: code.into(),
        title,
        detail,
        evidence,
    }
}

/// The USB-C spec limits a cable with no e-marker to 3 A, and so 60 W at 20 V.
const UNMARKED_CABLE_WATTS: u32 = 60;

pub fn diagnose(s: &Snapshot) -> Report {
    let mut out = Vec::new();
    let identity_visible = s.capabilities.cable_identity.level == Level::Full;
    for p in s.ports.iter().filter(|p| p.connected) {
        charging(p, identity_visible, &mut out);
        if let Some(c) = &p.cable {
            trust(p, c, &mut out);
        }
    }
    for d in s.usb_devices.iter().filter(|d| !d.is_hub) {
        let port = d
            .port_id
            .as_deref()
            .and_then(|id| s.ports.iter().find(|p| p.id == id));
        data_link(d, port, &mut out);
    }
    if let Some(power) = &s.power {
        measured_power(power, s, &mut out);
    }
    out.sort_by_key(|f| std::cmp::Reverse(f.severity));
    Report {
        snapshot: s.clone(),
        findings: out,
    }
}

fn charging(p: &Port, identity_visible: bool, out: &mut Vec<Finding>) {
    let id = Some(p.id.as_str());
    let Some(charger) = p.source_pdos.iter().map(|o| o.max_mw / 1000).max() else {
        return;
    };
    let (cable_w, cable_evidence) = match &p.cable {
        Some(c) => (c.max_watts, Evidence::Reported),
        None if identity_visible => (Some(UNMARKED_CABLE_WATTS), Evidence::Inferred),
        None => (None, Evidence::Inferred),
    };
    if let Some(cable) = cable_w.filter(|c| *c < charger) {
        let how = if p.cable.is_some() {
            format!("The cable's chip states a {cable} W limit.")
        } else {
            format!(
                "This cable has no e-marker chip, which limits it to {cable} W under the USB-C spec."
            )
        };
        out.push(finding(
            id,
            Severity::Warn,
            "cable_limits_charging",
            "Cable is limiting charging speed".into(),
            format!(
                "{how} The charger can supply {charger} W. A higher-rated cable would allow more."
            ),
            cable_evidence,
        ));
    }
    let ceiling = cable_w.map_or(charger, |c| c.min(charger));
    if let Some(active) = &p.active_pdo {
        let now = active.max_mw / 1000;
        if (now as f64) < ceiling as f64 * 0.8 {
            out.push(finding(
                id,
                Severity::Info,
                "negotiated_below_limit",
                format!("Charging contract is {now} W (up to {ceiling} W is possible)"),
                "The charger and cable allow more. Devices usually ask for less when the battery is nearly full or the device needs less.".into(),
                Evidence::Reported,
            ));
        } else {
            out.push(finding(
                id,
                Severity::Good,
                "charging_at_limit",
                format!("Charging contract is {now} W, the most this setup allows"),
                format!("The charger offers up to {charger} W and nothing in the chain is holding it back."),
                Evidence::Reported,
            ));
        }
    }
}

fn trust(p: &Port, c: &CableInfo, out: &mut Vec<Finding>) {
    let mut oddities = Vec::new();
    if c.vendor_id == 0 {
        oddities.push("its vendor ID is zero".to_string());
    } else if c.vendor_name.is_none() {
        oddities.push(format!("vendor ID {:04x} is not in our list", c.vendor_id));
    }
    if c.speed == CableSpeed::Reserved {
        oddities.push("its speed field holds a reserved value".to_string());
    }
    if c.current_a.is_none() {
        oddities.push("its current field holds a reserved value".to_string());
    }
    if !oddities.is_empty() {
        out.push(finding(
            Some(&p.id),
            Severity::Info,
            "unusual_emarker",
            "This cable's chip reports unusual values".into(),
            format!(
                "{}. This is common on budget cables and does not prove a fault, but treat its stated ratings with caution.",
                capitalise(&oddities.join("; "))
            ),
            Evidence::Reported,
        ));
    }
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map_or_else(String::new, |f| f.to_uppercase().chain(chars).collect())
}

fn device_name(d: &UsbDevice) -> String {
    d.product_name
        .clone()
        .or_else(|| d.vendor_name.clone())
        .unwrap_or_else(|| format!("USB device {:04x}:{:04x}", d.vendor_id, d.product_id))
}

fn data_link(d: &UsbDevice, port: Option<&Port>, out: &mut Vec<Finding>) {
    let Some(now) = d.speed_mbps else { return };
    let id = port.map(|p| p.id.as_str());
    let name = device_name(d);
    let cable = port.and_then(|p| p.cable.as_ref());
    let cable_mbps = cable.and_then(|c| c.speed.mbps());

    if let Some(capable) = d.capable_mbps.filter(|c| *c > now) {
        // Only blame the cable when its own stated rating explains the speed.
        let blame_cable = cable_mbps.is_some_and(|c| c < capable && c <= now);
        let (code, title, detail, evidence) = if blame_cable {
            (
                "cable_limits_data",
                "Cable is limiting data speed".to_string(),
                format!(
                    "{name} supports {} but the cable is rated for {}.",
                    speed_words(capable),
                    speed_words(cable_mbps.unwrap_or(now))
                ),
                Evidence::Reported,
            )
        } else {
            (
                "link_below_device",
                format!("{name} is running slower than it can"),
                format!(
                    "It supports {} but is running at {}. The cause is usually the cable, a hub, or the port. Try a known fast cable in another port to tell which.",
                    speed_words(capable),
                    speed_words(now)
                ),
                Evidence::Inferred,
            )
        };
        out.push(finding(id, Severity::Warn, code, title, detail, evidence));
    } else if d.capable_mbps.is_some() {
        out.push(finding(
            id,
            Severity::Good,
            "link_at_device_max",
            format!("{name} is at full speed ({})", speed_words(now)),
            "That is the fastest this device supports, so the cable is not holding it back.".into(),
            Evidence::Reported,
        ));
    }
}

fn measured_power(power: &PowerReading, s: &Snapshot, out: &mut Vec<Finding>) {
    let watts = power.milliwatts.map(|mw| mw as f64 / 1000.0);
    match (power.state, watts) {
        (ChargeState::Full, _) => out.push(finding(
            None,
            Severity::Info,
            "battery_full",
            "Battery is full, so little power is being drawn".into(),
            "Charging speed cannot be judged right now. Test again with the battery below 80%.".into(),
            Evidence::Reported,
        )),
        (ChargeState::Charging, Some(w)) => {
            let no_contract = !s.ports.iter().any(|p| p.active_pdo.is_some());
            let tail = if no_contract {
                " This system does not reveal what the charger or cable are rated for, so run the charge test with a known good cable and compare."
            } else {
                ""
            };
            out.push(finding(
                None,
                Severity::Info,
                "measured_charge_rate",
                format!("Battery is charging at about {w:.1} W"),
                format!("Measured at the battery, so it reads lower than the charger's rating.{tail}"),
                Evidence::Measured,
            ));
        }
        (ChargeState::NotCharging, _) if power.external_power == Some(true) => out.push(finding(
            None,
            Severity::Warn,
            "plugged_not_charging",
            "Plugged in, but the battery is not charging".into(),
            "The system may be managing battery health, or the cable or charger may not be supplying enough power.".into(),
            Evidence::Reported,
        )),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pd::decode_cable;

    fn snap(ports: Vec<Port>, identity: Level) -> Snapshot {
        Snapshot {
            capabilities: Capabilities {
                cable_identity: Access {
                    level: identity,
                    why: String::new(),
                },
                ..Default::default()
            },
            ports,
            ..Default::default()
        }
    }

    fn codes(r: &Report) -> Vec<&str> {
        r.findings.iter().map(|f| f.code.as_str()).collect()
    }

    fn charging_port(cable: Option<CableInfo>, active_w: u32) -> Port {
        Port {
            id: "p0".into(),
            connected: true,
            cable,
            source_pdos: vec![Pdo::fixed(5000, 3000), Pdo::fixed(20000, 5000)],
            active_pdo: Some(Pdo::fixed(20000, active_w * 50)),
            ..Default::default()
        }
    }

    fn cable_3a() -> CableInfo {
        decode_cable((0b011 << 27) | 0x05ac, 0, 0, 0b01 << 5)
    }

    fn cable_5a() -> CableInfo {
        decode_cable((0b011 << 27) | 0x05ac, 0, 0, (0b10 << 5) | 0b011)
    }

    #[test]
    fn three_amp_cable_on_100w_charger_blames_cable() {
        let r = diagnose(&snap(
            vec![charging_port(Some(cable_3a()), 60)],
            Level::Full,
        ));
        assert_eq!(r.findings[0].code, "cable_limits_charging");
        assert_eq!(r.findings[0].evidence, Evidence::Reported);
        assert!(codes(&r).contains(&"charging_at_limit"));
    }

    #[test]
    fn unmarked_cable_is_inferred_only_when_identity_is_visible() {
        let r = diagnose(&snap(vec![charging_port(None, 60)], Level::Full));
        assert_eq!(r.findings[0].code, "cable_limits_charging");
        assert_eq!(r.findings[0].evidence, Evidence::Inferred);
        // On an OS that hides e-markers a missing cable record proves nothing.
        let r = diagnose(&snap(vec![charging_port(None, 60)], Level::None));
        assert!(!codes(&r).contains(&"cable_limits_charging"));
    }

    #[test]
    fn low_contract_with_good_cable_is_not_a_cable_problem() {
        let r = diagnose(&snap(
            vec![charging_port(Some(cable_5a()), 30)],
            Level::Full,
        ));
        assert_eq!(codes(&r), vec!["negotiated_below_limit"]);
    }

    #[test]
    fn disconnected_ports_are_silent() {
        let mut p = charging_port(Some(cable_3a()), 60);
        p.connected = false;
        assert!(diagnose(&snap(vec![p], Level::Full)).findings.is_empty());
    }

    #[test]
    fn flags_zero_vendor_and_reserved_bits() {
        let c = decode_cable(0b011 << 27, 0, 0, (0b11 << 5) | 0b111);
        let r = diagnose(&snap(
            vec![Port {
                id: "p".into(),
                connected: true,
                cable: Some(c),
                ..Default::default()
            }],
            Level::Full,
        ));
        let f = r
            .findings
            .iter()
            .find(|f| f.code == "unusual_emarker")
            .unwrap();
        assert!(f.detail.contains("vendor ID is zero"));
        assert!(f.detail.contains("speed"));
        assert!(f.detail.contains("current"));
    }

    fn dev(now: u32, capable: Option<u32>) -> UsbDevice {
        UsbDevice {
            speed_mbps: Some(now),
            capable_mbps: capable,
            port_id: Some("p0".into()),
            ..Default::default()
        }
    }

    #[test]
    fn data_speed_blame_needs_cable_evidence() {
        let mut s = snap(
            vec![Port {
                id: "p0".into(),
                connected: true,
                cable: Some(cable_3a()),
                ..Default::default()
            }],
            Level::Full,
        );
        s.usb_devices = vec![dev(480, Some(10_000))];
        assert!(codes(&diagnose(&s)).contains(&"cable_limits_data"));

        // A 40 Gbps cable cannot be why a 10 Gbps device runs at 480 Mbps.
        s.ports[0].cable = Some(cable_5a());
        let r = diagnose(&s);
        assert!(codes(&r).contains(&"link_below_device"));
        assert!(!codes(&r).contains(&"cable_limits_data"));

        s.usb_devices = vec![dev(10_000, Some(10_000))];
        assert!(codes(&diagnose(&s)).contains(&"link_at_device_max"));
    }

    #[test]
    fn hubs_and_unknown_capability_produce_no_data_verdict() {
        let mut s = snap(vec![], Level::None);
        s.usb_devices = vec![
            UsbDevice {
                is_hub: true,
                ..dev(480, Some(5000))
            },
            dev(480, None),
        ];
        assert!(diagnose(&s).findings.is_empty());
    }

    #[test]
    fn measured_power_states() {
        let mut s = snap(vec![], Level::None);
        s.power = Some(PowerReading {
            state: ChargeState::Charging,
            milliwatts: Some(18_500),
            ..Default::default()
        });
        let r = diagnose(&s);
        assert_eq!(r.findings[0].evidence, Evidence::Measured);
        assert!(r.findings[0].detail.contains("known good cable"));
        s.power = Some(PowerReading {
            state: ChargeState::NotCharging,
            external_power: Some(true),
            ..Default::default()
        });
        assert_eq!(codes(&diagnose(&s)), vec!["plugged_not_charging"]);
        s.power = Some(PowerReading {
            state: ChargeState::Full,
            ..Default::default()
        });
        assert_eq!(codes(&diagnose(&s)), vec!["battery_full"]);
    }

    #[test]
    fn worst_finding_comes_first() {
        let r = diagnose(&snap(
            vec![charging_port(Some(cable_3a()), 60)],
            Level::Full,
        ));
        assert!(
            r.findings
                .windows(2)
                .all(|w| w[0].severity >= w[1].severity)
        );
    }
}
