//! macOS: walks the IORegistry. Public IOKit calls only; no SMC, no daemon.
use crate::macos_parse::*;
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetTypeID, CFArrayGetValueAtIndex};
use core_foundation_sys::base::{CFGetTypeID, CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::{CFDataGetBytePtr, CFDataGetLength, CFDataGetTypeID};
use core_foundation_sys::dictionary::{
    CFDictionaryGetCount, CFDictionaryGetKeysAndValues, CFDictionaryGetTypeID,
};
use core_foundation_sys::number::{
    CFBooleanGetTypeID, CFBooleanGetValue, CFNumberGetTypeID, CFNumberGetValue, kCFNumberSInt64Type,
};
use core_foundation_sys::set::{CFSetGetCount, CFSetGetTypeID, CFSetGetValues};
use core_foundation_sys::string::CFStringGetTypeID;
use io_kit_sys::types::{io_iterator_t, io_object_t};
use io_kit_sys::{
    IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperty, IOServiceGetMatchingServices,
    IOServiceMatching, kIOMasterPortDefault,
};
use qloudicable_core::*;
use std::ffi::{CString, c_void};

/// Registry names can differ per chip family, so all known ones are tried.
const PORT_KEYS: [&str; 5] = [
    "PortType",
    "PortTypeDescription",
    "PortNumber",
    "ConnectionActive",
    "TransportsActive",
];
const PARENT_KEYS: [&str; 6] = [
    "ParentBuiltInPortType",
    "ParentPortType",
    "ParentBuiltInPortNumber",
    "ParentPortNumber",
    "Priority",
    "Metadata",
];
const POWER_KEYS: [&str; 7] = [
    "ParentBuiltInPortType",
    "ParentPortType",
    "ParentBuiltInPortNumber",
    "ParentPortNumber",
    "Priority",
    "PowerSourceOptions",
    "WinningPowerSourceOption",
];
const USB_KEYS: [&str; 8] = [
    "idVendor",
    "idProduct",
    "bcdUSB",
    "Device Speed",
    "USB Vendor Name",
    "USB Product Name",
    "bDeviceClass",
    "locationID",
];
const BATTERY_KEYS: [&str; 6] = [
    "Voltage",
    "Amperage",
    "IsCharging",
    "ExternalConnected",
    "FullyCharged",
    "CurrentCapacity",
];

const PORT_CLASSES: [&str; 7] = [
    "AppleHPMInterfaceType10",
    "AppleHPMInterfaceType11",
    "AppleHPMInterfaceType12",
    "AppleHPMInterfaceType18",
    "AppleTCControllerType10",
    "AppleTCControllerType11",
    // Shared superclass: a catch-all for hardware not named above. Duplicates of
    // the classes above are removed in collect().
    "IOPort",
];

unsafe fn convert(v: CFTypeRef) -> Option<Value> {
    unsafe {
        let t = CFGetTypeID(v);
        if t == CFStringGetTypeID() {
            let s = CFString::wrap_under_get_rule(v as _);
            Some(Value::Str(s.to_string()))
        } else if t == CFNumberGetTypeID() {
            let mut n: i64 = 0;
            CFNumberGetValue(
                v as _,
                kCFNumberSInt64Type,
                &mut n as *mut i64 as *mut c_void,
            );
            Some(Value::Num(n))
        } else if t == CFBooleanGetTypeID() {
            Some(Value::Bool(CFBooleanGetValue(v as _)))
        } else if t == CFDataGetTypeID() {
            let len = CFDataGetLength(v as _) as usize;
            Some(Value::Data(
                std::slice::from_raw_parts(CFDataGetBytePtr(v as _), len).to_vec(),
            ))
        } else if t == CFArrayGetTypeID() {
            let n = CFArrayGetCount(v as _);
            Some(Value::List(
                (0..n)
                    .filter_map(|i| convert(CFArrayGetValueAtIndex(v as _, i)))
                    .collect(),
            ))
        } else if t == CFSetGetTypeID() {
            // PowerSourceOptions is published as a set rather than an array.
            let n = CFSetGetCount(v as _) as usize;
            let mut items = vec![std::ptr::null::<c_void>(); n];
            CFSetGetValues(v as _, items.as_mut_ptr());
            Some(Value::List(
                items.into_iter().filter_map(|i| convert(i)).collect(),
            ))
        } else if t == CFDictionaryGetTypeID() {
            let n = CFDictionaryGetCount(v as _) as usize;
            let mut keys = vec![std::ptr::null::<c_void>(); n];
            let mut vals = vec![std::ptr::null::<c_void>(); n];
            CFDictionaryGetKeysAndValues(v as _, keys.as_mut_ptr(), vals.as_mut_ptr());
            let mut out = Vec::with_capacity(n);
            for (k, val) in keys.into_iter().zip(vals) {
                if CFGetTypeID(k) == CFStringGetTypeID()
                    && let Some(c) = convert(val)
                {
                    out.push((CFString::wrap_under_get_rule(k as _).to_string(), c));
                }
            }
            Some(Value::Dict(out))
        } else {
            None
        }
    }
}

/// Reads only the named keys, one at a time. The bulk call,
/// IORegistryEntryCreateCFProperties, can abort the process if a service is torn
/// down mid-read, as happens when a cable is unplugged.
fn services(class: &str, keys: &[&str]) -> Vec<Value> {
    let mut out = Vec::new();
    let Ok(name) = CString::new(class) else {
        return out;
    };
    unsafe {
        let matching = IOServiceMatching(name.as_ptr());
        if matching.is_null() {
            return out;
        }
        let mut iter: io_iterator_t = 0;
        // This call consumes `matching`.
        if IOServiceGetMatchingServices(kIOMasterPortDefault, matching as _, &mut iter) != 0 {
            return out;
        }
        loop {
            let entry: io_object_t = IOIteratorNext(iter);
            if entry == 0 {
                break;
            }
            let mut props = Vec::new();
            for key in keys {
                let cf_key = CFString::new(key);
                let v = IORegistryEntryCreateCFProperty(
                    entry,
                    cf_key.as_concrete_TypeRef(),
                    kCFAllocatorDefault,
                    0,
                );
                if !v.is_null() {
                    if let Some(c) = convert(v as CFTypeRef) {
                        props.push((key.to_string(), c));
                    }
                    CFRelease(v as CFTypeRef);
                }
            }
            out.push(Value::Dict(props));
            IOObjectRelease(entry);
        }
        IOObjectRelease(iter);
    }
    out
}

fn battery() -> Option<PowerReading> {
    let b = services("AppleSmartBattery", &BATTERY_KEYS)
        .into_iter()
        .next()?;
    let voltage_mv = b.num("Voltage").map(|v| v as u32);
    // Registry integers can be published unsigned; negative amperage wraps.
    let amperage = b.num("Amperage").map(|a| a as i32 as i64);
    let current_ma = amperage.map(|a| a.unsigned_abs() as u32);
    let charging = b.flag("IsCharging").unwrap_or(false);
    let external = b.flag("ExternalConnected");
    let full = b.flag("FullyCharged").unwrap_or(false);
    Some(PowerReading {
        state: match (full, charging, external) {
            (true, ..) => ChargeState::Full,
            (_, true, _) => ChargeState::Charging,
            (_, false, Some(true)) => ChargeState::NotCharging,
            (_, false, Some(false)) => ChargeState::Discharging,
            _ => ChargeState::Unknown,
        },
        external_power: external,
        voltage_mv,
        current_ma,
        milliwatts: voltage_mv.zip(current_ma).map(|(v, i)| v * i / 1000),
        battery_percent: b.num("CurrentCapacity").map(|c| c.clamp(0, 100) as u8),
        source: "AppleSmartBattery".into(),
    })
}

pub fn collect() -> Snapshot {
    let mut ports: Vec<Port> = Vec::new();
    for (i, v) in PORT_CLASSES
        .iter()
        .flat_map(|c| services(c, &PORT_KEYS))
        .enumerate()
    {
        if let Some(p) = parse_port(&v, i)
            && !ports.iter().any(|known| known.id == p.id)
        {
            ports.push(p);
        }
    }
    ports.sort_by(|a, b| a.label.cmp(&b.label));
    let find = |ports: &mut Vec<Port>, key: (i64, i64)| {
        let id = format!("{}:{}", key.0, key.1);
        ports.iter().position(|p| p.id == id)
    };

    for v in services("IOPortTransportComponentCCUSBPDSOPp", &PARENT_KEYS) {
        if let Some(i) = find(&mut ports, parent_port(&v)) {
            ports[i].cable = parse_cable(&v);
        }
    }
    for v in services("IOPortTransportComponentCCUSBPDSOP", &PARENT_KEYS) {
        if let Some(i) = find(&mut ports, parent_port(&v)) {
            ports[i].partner = parse_partner(&v);
        }
    }
    for v in services("IOPortFeaturePowerSource", &POWER_KEYS) {
        if let Some(i) = find(&mut ports, parent_port(&v)) {
            ports[i].source_pdos = parse_pdos(&v, "PowerSourceOptions");
            ports[i].active_pdo = v.get("WinningPowerSourceOption").and_then(parse_pdo);
        }
    }

    let mut usb_devices: Vec<UsbDevice> = services("IOUSBHostDevice", &USB_KEYS)
        .iter()
        .filter_map(parse_usb_device)
        .collect();

    for dev in &mut usb_devices {
        if let Some(ref pid) = dev.port_id {
            if !ports.iter().any(|p| p.id == *pid) {
                let connected_ports: Vec<&Port> = ports.iter().filter(|p| p.connected).collect();
                if connected_ports.len() == 1 {
                    dev.port_id = Some(connected_ports[0].id.clone());
                } else {
                    dev.port_id = None;
                }
            }
        } else {
            let connected_ports: Vec<&Port> = ports.iter().filter(|p| p.connected).collect();
            if connected_ports.len() == 1 {
                dev.port_id = Some(connected_ports[0].id.clone());
            }
        }
    }
    let power = battery();
    let has_ports = !ports.is_empty();
    let intel = "Macs without Apple silicon don't publish USB-C port data.";
    Snapshot {
        platform: Platform::Macos,
        capabilities: Capabilities {
            cable_identity: if has_ports {
                Access::full()
            } else {
                Access::new(Level::None, intel)
            },
            charger_profiles: if has_ports {
                Access::full()
            } else {
                Access::new(Level::None, intel)
            },
            link_speed: Access::full(),
            power_measurement: if power.as_ref().is_some_and(|p| p.milliwatts.is_some()) {
                Access::full()
            } else {
                Access::new(
                    Level::None,
                    "No battery reading (desktop Macs have no battery).",
                )
            },
        },
        ports,
        usb_devices,
        power,
        ..Default::default()
    }
}
