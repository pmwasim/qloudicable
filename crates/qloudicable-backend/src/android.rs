//! Android: facts come from Kotlin (BatteryManager, UsbManager) as JSON, and the
//! sysfs reader is tried too, because a few devices let apps read typec.
use qloudicable_core::vendors;
use qloudicable_core::*;
use serde::Deserialize;

#[derive(Deserialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AndroidFacts {
    pub sdk_int: u32,
    pub plugged: Option<String>,
    pub status: Option<String>,
    pub voltage_mv: Option<u32>,
    pub current_ua: Option<i64>,
    pub battery_percent: Option<u8>,
    pub usb_devices: Vec<AndroidUsb>,
}

#[derive(Deserialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AndroidUsb {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub version: Option<String>,
    pub device_class: u8,
}

pub fn snapshot(f: AndroidFacts, sysfs: Snapshot) -> Snapshot {
    // Current's sign and even its units differ between phone makers. Values under
    // 20 are almost surely amps-scale noise, so they count as no reading.
    let current_ma = f
        .current_ua
        .map(|u| (u.unsigned_abs() / 1000) as u32)
        .filter(|ma| *ma >= 20);
    let milliwatts = f.voltage_mv.zip(current_ma).map(|(v, i)| v * i / 1000);
    let plugged = f.plugged.as_deref().filter(|p| *p != "none");
    let power = PowerReading {
        state: match f.status.as_deref() {
            Some("charging") => ChargeState::Charging,
            Some("discharging") => ChargeState::Discharging,
            Some("full") => ChargeState::Full,
            Some("not_charging") => ChargeState::NotCharging,
            _ => ChargeState::Unknown,
        },
        external_power: Some(plugged.is_some()),
        voltage_mv: f.voltage_mv,
        current_ma,
        milliwatts,
        battery_percent: f.battery_percent,
        source: format!("BatteryManager ({})", plugged.unwrap_or("unplugged")),
    };
    let usb_devices = f
        .usb_devices
        .into_iter()
        .map(|d| UsbDevice {
            vendor_id: d.vendor_id,
            product_id: d.product_id,
            vendor_name: d
                .manufacturer
                .or_else(|| vendors::name(d.vendor_id).map(str::to_string)),
            product_name: d.product,
            capable_mbps: d.version.as_deref().and_then(crate::capable_from_version),
            usb_version: d.version,
            speed_mbps: None,
            port_id: None,
            is_hub: d.device_class == 9,
        })
        .collect();

    let has_typec = !sysfs.ports.is_empty();
    let hidden = "Android keeps this from normal apps.";
    Snapshot {
        tool_version: qloudicable_core::version_label(),
        platform: Platform::Android,
        capabilities: Capabilities {
            cable_identity: if has_typec {
                sysfs.capabilities.cable_identity.clone()
            } else {
                Access::new(Level::None, hidden)
            },
            charger_profiles: if has_typec {
                sysfs.capabilities.charger_profiles.clone()
            } else {
                Access::new(Level::None, hidden)
            },
            link_speed: Access::new(
                Level::None,
                "Android lists USB devices but not their current speed.",
            ),
            power_measurement: if milliwatts.is_some() {
                Access::new(
                    Level::Partial,
                    "Measured at the battery. Accuracy varies by phone.",
                )
            } else {
                Access::new(Level::None, "This phone gives no battery current reading.")
            },
        },
        ports: sysfs.ports,
        usb_devices,
        power: Some(power),
        notes: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kotlin_json_becomes_a_snapshot() {
        let f: AndroidFacts = serde_json::from_str(
            r#"{"sdkInt":34,"plugged":"ac","status":"charging","voltageMv":4200,"currentUa":-2000000,"batteryPercent":40,
                "usbDevices":[{"vendorId":1452,"productId":5,"product":"Drive","version":"3.10","deviceClass":0}]}"#,
        )
        .unwrap();
        let s = snapshot(f, Snapshot::default());
        let p = s.power.as_ref().unwrap();
        assert_eq!(p.state, ChargeState::Charging);
        assert_eq!(p.milliwatts, Some(8400));
        assert_eq!(s.usb_devices[0].vendor_name.as_deref(), Some("Apple, Inc."));
        assert_eq!(s.usb_devices[0].capable_mbps, Some(5000));
        assert_eq!(s.capabilities.cable_identity.level, Level::None);
        assert!(
            diagnose(&s)
                .findings
                .iter()
                .any(|f| f.code == "measured_charge_rate")
        );
    }

    #[test]
    fn unplugged_and_missing_current() {
        let f = AndroidFacts {
            plugged: Some("none".into()),
            voltage_mv: Some(4000),
            current_ua: Some(0),
            ..Default::default()
        };
        let s = snapshot(f, Snapshot::default());
        let p = s.power.as_ref().unwrap();
        assert_eq!(p.external_power, Some(false));
        assert_eq!(p.milliwatts, None);
        assert_eq!(s.capabilities.power_measurement.level, Level::None);
    }
}
