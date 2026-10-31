//! Converts IOKit property values into the model, kept free of FFI so it compiles
//! and is tested everywhere. Property names follow the IORegistry services listed
//! in docs/DESIGN.md.
use qloudicable_core::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num(i64),
    Str(String),
    Bool(bool),
    Data(Vec<u8>),
    List(Vec<Value>),
    Dict(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Dict(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn num(&self, key: &str) -> Option<i64> {
        match self.get(key)? {
            Value::Num(n) => Some(*n),
            Value::Bool(b) => Some(*b as i64),
            _ => None,
        }
    }
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn flag(&self, key: &str) -> Option<bool> {
        self.num(key).map(|n| n != 0)
    }
}

/// (type, number) pair that ties a transport or power service to its port.
pub fn parent_port(v: &Value) -> (i64, i64) {
    let ty = v.num("ParentBuiltInPortType").or_else(|| v.num("ParentPortType")).unwrap_or(0);
    let number = v
        .num("ParentBuiltInPortNumber")
        .or_else(|| v.num("ParentPortNumber"))
        .or_else(|| v.num("Priority").map(|p| p & 0xFF))
        .unwrap_or(0);
    (ty, number)
}

pub fn parse_pdo(v: &Value) -> Option<Pdo> {
    let mv = v.num("Voltage (mV)")? as u32;
    let ma = v.num("Max Current (mA)").unwrap_or(0) as u32;
    let mut pdo = Pdo::fixed(mv, ma);
    if let Some(mw) = v.num("Max Power (mW)") {
        pdo.max_mw = mw as u32;
    }
    Some(pdo)
}

pub fn parse_pdos(v: &Value, key: &str) -> Vec<Pdo> {
    let mut out: Vec<Pdo> = match v.get(key) {
        Some(Value::List(items)) => items.iter().filter_map(parse_pdo).collect(),
        _ => vec![],
    };
    out.sort_by_key(|p| p.max_mw);
    out
}

/// VDOs arrive as a list of little-endian 4-byte blobs under `Metadata`.
pub fn parse_vdos(v: &Value) -> Vec<u32> {
    match v.get("Metadata").and_then(|m| m.get("VDOs")) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|i| match i {
                Value::Data(d) if d.len() >= 4 => Some(u32::from_le_bytes([d[0], d[1], d[2], d[3]])),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

/// Discover Identity order: ID header, cert stat, product, product-type VDOs.
pub fn parse_cable(v: &Value) -> Option<CableInfo> {
    let vdos = parse_vdos(v);
    let get = |i: usize| vdos.get(i).copied().unwrap_or(0);
    (!vdos.is_empty() && get(0) != 0).then(|| pd::decode_cable(get(0), get(1), get(2), get(3)))
}

pub fn parse_partner(v: &Value) -> Option<Partner> {
    let vdos = parse_vdos(v);
    let header = *vdos.first()?;
    let id = pd::vid(header);
    Some(Partner {
        vendor_id: Some(id),
        vendor_name: vendors::name(id).map(str::to_string),
        product_id: vdos.get(2).map(|p| pd::pid(*p)),
        kind: pd::partner_kind(header).map(str::to_string),
        supports_pd: Some(true),
    })
}

/// `Device Speed` uses IOKit's enum: 0 low, 1 full, 2 high, 3 super, 4 super+,
/// 5 super+ by 2.
pub fn speed_mbps(raw: i64) -> Option<u32> {
    match raw {
        0 => Some(2),
        1 => Some(12),
        2 => Some(480),
        3 => Some(5_000),
        4 => Some(10_000),
        5 => Some(20_000),
        _ => None,
    }
}

pub fn parse_usb_device(v: &Value) -> Option<UsbDevice> {
    let vendor_id = v.num("idVendor")? as u16;
    let bcd = v.num("bcdUSB");
    let usb_version = bcd.map(|b| format!("{:x}.{:02x}", b >> 8, b & 0xFF));
    Some(UsbDevice {
        vendor_id,
        product_id: v.num("idProduct").unwrap_or(0) as u16,
        vendor_name: v.text("USB Vendor Name").map(str::to_string).or_else(|| vendors::name(vendor_id).map(str::to_string)),
        product_name: v.text("USB Product Name").map(str::to_string),
        capable_mbps: usb_version.as_deref().and_then(crate::capable_from_version),
        usb_version,
        speed_mbps: v.num("Device Speed").and_then(speed_mbps),
        port_id: None,
        is_hub: v.num("bDeviceClass") == Some(9),
    })
}

/// Real physical ports carry `PortTypeDescription`; other IOPort nodes don't.
pub fn parse_port(v: &Value, index: usize) -> Option<Port> {
    v.text("PortTypeDescription")?;
    let ty = v.num("PortType").unwrap_or(0);
    let number = v.num("PortNumber").unwrap_or(index as i64 + 1);
    let transports = match v.get("TransportsActive") {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|t| match t {
                Value::Str(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => vec![],
    };
    Some(Port {
        id: format!("{ty}:{number}"),
        label: v.text("PortDescription").map_or_else(|| format!("USB-C port {number}"), str::to_string),
        connected: v.flag("ConnectionActive").unwrap_or(false),
        transports,
        ..Default::default()
    })
}

use qloudicable_core::pd;
use qloudicable_core::vendors;

#[cfg(test)]
mod tests {
    use super::*;

    fn d(items: Vec<(&str, Value)>) -> Value {
        Value::Dict(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    #[test]
    fn cable_from_vdo_blobs() {
        let blob = |x: u32| Value::Data(x.to_le_bytes().to_vec());
        let header: u32 = (0b011 << 27) | 0x05AC;
        let v = d(vec![(
            "Metadata",
            d(vec![("VDOs", Value::List(vec![blob(header), blob(7), blob(0x1234_0100), blob((0b10 << 5) | 0b011)]))]),
        )]);
        let c = parse_cable(&v).unwrap();
        assert_eq!((c.vendor_id, c.product_id, c.xid), (0x05AC, 0x1234, 7));
        assert_eq!(c.max_watts, Some(100));
        assert!(parse_cable(&d(vec![])).is_none());
    }

    #[test]
    fn pdo_lists_sort_and_keep_stated_power() {
        let opt = |mv, ma, mw| d(vec![("Voltage (mV)", Value::Num(mv)), ("Max Current (mA)", Value::Num(ma)), ("Max Power (mW)", Value::Num(mw))]);
        let v = d(vec![("PowerSourceOptions", Value::List(vec![opt(20000, 4700, 94000), opt(5000, 3000, 15000)]))]);
        let p = parse_pdos(&v, "PowerSourceOptions");
        assert_eq!(p[0].max_mw, 15000);
        assert_eq!(p[1].max_mw, 94000);
        assert!(parse_pdos(&d(vec![]), "PowerSourceOptions").is_empty());
    }

    #[test]
    fn parent_port_prefers_built_in() {
        let v = d(vec![("ParentBuiltInPortType", Value::Num(2)), ("ParentPortType", Value::Num(9)), ("ParentPortNumber", Value::Num(3))]);
        assert_eq!(parent_port(&v), (2, 3));
        assert_eq!(parent_port(&d(vec![("Priority", Value::Num(0x105))])), (0, 5));
    }

    #[test]
    fn usb_device_fields() {
        let v = d(vec![("idVendor", Value::Num(0x05AC)), ("bcdUSB", Value::Num(0x0320)), ("Device Speed", Value::Num(2)), ("USB Product Name", Value::Str("X".into()))]);
        let u = parse_usb_device(&v).unwrap();
        assert_eq!(u.usb_version.as_deref(), Some("3.20"));
        assert_eq!((u.speed_mbps, u.capable_mbps), (Some(480), Some(5000)));
    }

    #[test]
    fn port_fields() {
        let v = d(vec![("PortTypeDescription", Value::Str("USB-C".into())), ("PortType", Value::Num(2)), ("PortNumber", Value::Num(1)), ("ConnectionActive", Value::Bool(true)), ("TransportsActive", Value::List(vec![Value::Str("USB3".into())]))]);
        let p = parse_port(&v, 0).unwrap();
        assert!(parse_port(&d(vec![("PortType", Value::Num(2))]), 0).is_none());
        assert_eq!(p.id, "2:1");
        assert!(p.connected);
        assert_eq!(p.transports, vec!["USB3"]);
    }
}
