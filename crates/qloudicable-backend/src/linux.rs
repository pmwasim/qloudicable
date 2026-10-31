//! Linux and Android sysfs reader: /sys/class/typec, /sys/class/usb_power_delivery,
//! /sys/class/power_supply and /sys/bus/usb. Every path hangs off `root`, so tests
//! run against a fake tree.
use crate::capable_from_version;
use std::fs;
use std::path::Path;
use qloudicable_core::pd;
use qloudicable_core::vendors;
use qloudicable_core::*;

fn read(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn hex32(p: &Path) -> Option<u32> {
    u32::from_str_radix(read(p)?.trim_start_matches("0x"), 16).ok()
}

fn hex16(p: &Path) -> Option<u16> {
    u16::from_str_radix(&read(p)?, 16).ok()
}

/// typec attributes mark the current value in brackets: "[source] sink".
fn bracketed(s: String) -> String {
    match (s.find('['), s.find(']')) {
        (Some(a), Some(b)) if a < b => s[a + 1..b].to_string(),
        _ => s,
    }
}

/// "9000mV" -> 9000. The kernel prints units in these attributes.
fn milli(p: &Path) -> Option<u32> {
    read(p)?.trim_end_matches(|c: char| !c.is_ascii_digit()).parse().ok()
}

fn entries(p: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(p)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn pdos(capabilities: &Path) -> Vec<Pdo> {
    entries(capabilities)
        .iter()
        .filter_map(|name| {
            let d = capabilities.join(name);
            // Entries look like "1:fixed_supply", "2:variable_supply", "3:battery",
            // "4:programmable_supply".
            let (_, kind) = name.split_once(':')?;
            Some(match kind {
                "fixed_supply" => Pdo::fixed(milli(&d.join("voltage"))?, milli(&d.join("maximum_current"))?),
                "variable_supply" | "programmable_supply" => {
                    let (min, max, ma) = (
                        milli(&d.join("minimum_voltage"))?,
                        milli(&d.join("maximum_voltage"))?,
                        milli(&d.join("maximum_current"))?,
                    );
                    Pdo {
                        kind: if kind == "variable_supply" { PdoKind::Variable } else { PdoKind::Pps },
                        min_mv: min,
                        max_mv: max,
                        max_ma: ma,
                        max_mw: max * ma / 1000,
                    }
                }
                "battery" => {
                    let (min, max, mw) = (
                        milli(&d.join("minimum_voltage"))?,
                        milli(&d.join("maximum_voltage"))?,
                        milli(&d.join("maximum_power"))?,
                    );
                    Pdo { kind: PdoKind::Battery, min_mv: min, max_mv: max, max_ma: mw * 1000 / max.max(1), max_mw: mw }
                }
                _ => return None,
            })
        })
        .collect()
}

fn read_port(typec: &Path, name: &str) -> Port {
    let dir = typec.join(name);
    let partner_dir = typec.join(format!("{name}-partner"));
    let cable_dir = typec.join(format!("{name}-cable"));
    let connected = partner_dir.exists();

    let cable = hex32(&cable_dir.join("identity/id_header")).filter(|h| *h != 0).map(|header| {
        pd::decode_cable(
            header,
            hex32(&cable_dir.join("identity/cert_stat")).unwrap_or(0),
            hex32(&cable_dir.join("identity/product")).unwrap_or(0),
            hex32(&cable_dir.join("identity/product_type_vdo1")).unwrap_or(0),
        )
    });

    let partner = connected.then(|| {
        let header = hex32(&partner_dir.join("identity/id_header")).filter(|h| *h != 0);
        let vendor_id = header.map(pd::vid);
        Partner {
            vendor_id,
            vendor_name: vendor_id.and_then(vendors::name).map(str::to_string),
            product_id: hex32(&partner_dir.join("identity/product")).map(pd::pid),
            kind: header.and_then(pd::partner_kind).map(str::to_string),
            supports_pd: read(&partner_dir.join("supports_usb_power_delivery")).map(|s| s == "yes"),
        }
    });

    let source_pdos = pdos(&partner_dir.join("usb_power_delivery/source-capabilities"));
    let number = name.trim_start_matches("port").parse::<u32>().map_or(0, |n| n + 1);
    Port {
        id: name.to_string(),
        label: format!("USB-C port {number}"),
        connected,
        data_role: read(&dir.join("data_role")).map(bracketed),
        power_role: read(&dir.join("power_role")).map(bracketed),
        orientation: read(&dir.join("orientation")),
        power_mode: read(&dir.join("power_operation_mode")),
        pd_revision: read(&dir.join("usb_power_delivery_revision")),
        cable,
        partner,
        source_pdos,
        active_pdo: None,
        transports: vec![],
    }
}

/// A typec port directory links its USB connectors as "usb3-port1". That maps to
/// device directories named "3-1", or "3-1.2" behind a hub.
fn port_for_device(typec: &Path, ports: &[Port], device: &str) -> Option<String> {
    let (bus, rest) = device.split_once('-')?;
    let root_port = rest.split('.').next()?;
    let wanted = format!("usb{bus}-port{root_port}");
    ports
        .iter()
        .find(|p| typec.join(&p.id).join(&wanted).exists())
        .map(|p| p.id.clone())
}

fn usb_devices(root: &Path, typec: &Path, ports: &[Port]) -> Vec<UsbDevice> {
    let base = root.join("sys/bus/usb/devices");
    entries(&base)
        .into_iter()
        // "usb1" is a root hub and "1-1:1.0" an interface; devices look like "1-1.2".
        .filter(|n| n.contains('-') && !n.contains(':'))
        .filter_map(|name| {
            let d = base.join(&name);
            let vendor_id = hex16(&d.join("idVendor"))?;
            let usb_version = read(&d.join("version"));
            Some(UsbDevice {
                vendor_id,
                product_id: hex16(&d.join("idProduct")).unwrap_or(0),
                vendor_name: read(&d.join("manufacturer")).or_else(|| vendors::name(vendor_id).map(str::to_string)),
                product_name: read(&d.join("product")),
                capable_mbps: usb_version.as_deref().and_then(capable_from_version),
                usb_version,
                speed_mbps: read(&d.join("speed")).and_then(|s| s.parse::<f64>().ok()).map(|m| m.round() as u32),
                port_id: port_for_device(typec, ports, &name),
                is_hub: read(&d.join("bDeviceClass")).as_deref() == Some("09"),
            })
        })
        .collect()
}

fn power(root: &Path) -> Option<PowerReading> {
    let base = root.join("sys/class/power_supply");
    let names = entries(&base);
    let external = names
        .iter()
        .map(|n| base.join(n))
        .filter(|d| read(&d.join("type")).is_some_and(|t| t != "Battery"))
        .filter_map(|d| read(&d.join("online")))
        .fold(None, |any, v| Some(any.unwrap_or(false) || v == "1"));
    let battery = names
        .iter()
        .map(|n| base.join(n))
        .find(|d| read(&d.join("type")).as_deref() == Some("Battery"));
    let Some(d) = battery else {
        return external.map(|e| PowerReading {
            external_power: Some(e),
            source: "power_supply".into(),
            ..Default::default()
        });
    };
    // sysfs reports micro-units, and sign differs between drivers.
    let micro = |f: &str| read(&d.join(f)).and_then(|s| s.parse::<i64>().ok()).map(|v| (v.unsigned_abs() / 1000) as u32);
    let voltage_mv = micro("voltage_now");
    let current_ma = micro("current_now");
    let milliwatts = micro("power_now").or_else(|| Some(voltage_mv? * current_ma? / 1000));
    Some(PowerReading {
        state: match read(&d.join("status")).as_deref() {
            Some("Charging") => ChargeState::Charging,
            Some("Discharging") => ChargeState::Discharging,
            Some("Full") => ChargeState::Full,
            Some("Not charging") => ChargeState::NotCharging,
            _ => ChargeState::Unknown,
        },
        external_power: external,
        voltage_mv,
        current_ma,
        milliwatts,
        battery_percent: read(&d.join("capacity")).and_then(|s| s.parse().ok()),
        source: "power_supply battery".into(),
    })
}

pub fn collect(root: &Path) -> Snapshot {
    let typec = root.join("sys/class/typec");
    let ports: Vec<Port> = entries(&typec)
        .iter()
        // Only "portN" itself; "port0-partner" and "port0-cable" belong to it.
        .filter(|n| n.starts_with("port") && !n.contains('-'))
        .map(|n| read_port(&typec, n))
        .collect();

    let usb_devices = usb_devices(root, &typec, &ports);
    let power = power(root);
    let has_ports = !ports.is_empty();
    let no_typec = "This computer has no USB-C ports that report to Linux (the typec class is empty). Desktop boards and some laptops don't expose it.";
    let capabilities = Capabilities {
        cable_identity: if has_ports {
            Access::new(
                Level::Partial,
                "Linux shows the cable's chip only if the port driver reads it. Some drivers never do, so a missing cable record doesn't prove the cable has no chip.",
            )
        } else {
            Access::new(Level::None, no_typec)
        },
        charger_profiles: if has_ports { Access::new(Level::Partial, "Needs a kernel and port driver that publish PD data.") } else { Access::new(Level::None, no_typec) },
        link_speed: Access::full(),
        power_measurement: match &power {
            Some(p) if p.milliwatts.is_some() => Access::full(),
            _ => Access::new(Level::None, "No battery with a power reading was found."),
        },
    };
    Snapshot {
        platform: Platform::Linux,
        capabilities,
        ports,
        usb_devices,
        power,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct Tree(PathBuf);

    impl Tree {
        fn new(name: &str) -> Self {
            let p = std::env::temp_dir().join(format!("qloudicable-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn put(&self, rel: &str, value: &str) {
            let f = self.0.join(rel);
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(f, format!("{value}\n")).unwrap();
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn laptop(t: &Tree) {
        let c = "sys/class/typec";
        t.put(&format!("{c}/port0/data_role"), "[host] device");
        t.put(&format!("{c}/port0/power_role"), "source [sink]");
        t.put(&format!("{c}/port0/orientation"), "normal");
        t.put(&format!("{c}/port0/power_operation_mode"), "usb_power_delivery");
        t.put(&format!("{c}/port0/usb3-port1/marker"), "");
        t.put(&format!("{c}/port1/data_role"), "[host] device");
        t.put(&format!("{c}/port0-partner/supports_usb_power_delivery"), "yes");
        t.put(&format!("{c}/port0-partner/identity/id_header"), "0x10000000");
        t.put(&format!("{c}/port0-cable/identity/id_header"), "0x180005ac");
        t.put(&format!("{c}/port0-cable/identity/cert_stat"), "0x00000042");
        t.put(&format!("{c}/port0-cable/identity/product"), "0x70010100");
        t.put(&format!("{c}/port0-cable/identity/product_type_vdo1"), "0x00080020");
        let caps = format!("{c}/port0-partner/usb_power_delivery/source-capabilities");
        t.put(&format!("{caps}/1:fixed_supply/voltage"), "5000mV");
        t.put(&format!("{caps}/1:fixed_supply/maximum_current"), "3000mA");
        t.put(&format!("{caps}/2:fixed_supply/voltage"), "20000mV");
        t.put(&format!("{caps}/2:fixed_supply/maximum_current"), "5000mA");
        t.put(&format!("{caps}/3:programmable_supply/minimum_voltage"), "3300mV");
        t.put(&format!("{caps}/3:programmable_supply/maximum_voltage"), "21000mV");
        t.put(&format!("{caps}/3:programmable_supply/maximum_current"), "3000mA");
        let u = "sys/bus/usb/devices";
        t.put(&format!("{u}/3-1/idVendor"), "0781");
        t.put(&format!("{u}/3-1/idProduct"), "5581");
        t.put(&format!("{u}/3-1/version"), " 3.20");
        t.put(&format!("{u}/3-1/speed"), "480");
        t.put(&format!("{u}/3-1/product"), "Ultra Drive");
        t.put(&format!("{u}/3-1/bDeviceClass"), "00");
        t.put(&format!("{u}/3-1:1.0/bInterfaceClass"), "08");
        t.put(&format!("{u}/usb3/idVendor"), "1d6b");
        let p = "sys/class/power_supply";
        t.put(&format!("{p}/AC/type"), "Mains");
        t.put(&format!("{p}/AC/online"), "1");
        t.put(&format!("{p}/BAT0/type"), "Battery");
        t.put(&format!("{p}/BAT0/status"), "Charging");
        t.put(&format!("{p}/BAT0/voltage_now"), "12000000");
        t.put(&format!("{p}/BAT0/current_now"), "-2500000");
        t.put(&format!("{p}/BAT0/capacity"), "54");
    }

    #[test]
    fn reads_a_full_laptop_tree() {
        let t = Tree::new("full");
        laptop(&t);
        let s = collect(&t.0);
        assert_eq!(s.ports.len(), 2);
        let p = &s.ports[0];
        assert!(p.connected);
        assert_eq!(p.label, "USB-C port 1");
        assert_eq!(p.power_role.as_deref(), Some("sink"));
        assert_eq!(p.data_role.as_deref(), Some("host"));
        let c = p.cable.as_ref().unwrap();
        assert_eq!(c.vendor_id, 0x05ac);
        assert_eq!(c.product_id, 0x7001);
        assert_eq!(c.max_watts, Some(60));
        assert_eq!(p.partner.as_ref().unwrap().kind.as_deref(), Some("USB peripheral"));
        assert_eq!(p.source_pdos.len(), 3);
        assert_eq!(p.source_pdos[1], Pdo::fixed(20000, 5000));
        assert_eq!(p.source_pdos[2].kind, PdoKind::Pps);
        assert!(!s.ports[1].connected);

        // Interface and root hub entries are skipped; the device maps to its port.
        assert_eq!(s.usb_devices.len(), 1);
        let d = &s.usb_devices[0];
        assert_eq!(d.port_id.as_deref(), Some("port0"));
        assert_eq!(d.usb_version.as_deref(), Some("3.20"));
        assert_eq!((d.speed_mbps, d.capable_mbps), (Some(480), Some(5000)));

        let pw = s.power.as_ref().unwrap();
        assert_eq!(pw.state, ChargeState::Charging);
        assert_eq!(pw.external_power, Some(true));
        assert_eq!(pw.milliwatts, Some(30_000));
        assert_eq!(pw.battery_percent, Some(54));
    }

    #[test]
    fn full_tree_diagnoses_end_to_end() {
        let t = Tree::new("diag");
        laptop(&t);
        let r = diagnose(&collect(&t.0));
        let codes: Vec<_> = r.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"cable_limits_charging"), "{codes:?}");
        assert!(codes.contains(&"cable_limits_data") || codes.contains(&"link_below_device"), "{codes:?}");
        assert!(codes.contains(&"measured_charge_rate"), "{codes:?}");
    }

    #[test]
    fn empty_machine_explains_itself() {
        let t = Tree::new("empty");
        let s = collect(&t.0);
        assert!(s.ports.is_empty() && s.power.is_none());
        assert_eq!(s.capabilities.cable_identity.level, Level::None);
        assert!(s.capabilities.cable_identity.why.contains("typec"));
    }

    #[test]
    fn zeroed_identity_counts_as_no_cable_record() {
        let t = Tree::new("zero");
        t.put("sys/class/typec/port0/data_role", "[host] device");
        t.put("sys/class/typec/port0-partner/supports_usb_power_delivery", "yes");
        t.put("sys/class/typec/port0-cable/identity/id_header", "0x00000000");
        let s = collect(&t.0);
        assert!(s.ports[0].cable.is_none());
        assert!(s.ports[0].partner.as_ref().unwrap().kind.is_none());
    }
}
