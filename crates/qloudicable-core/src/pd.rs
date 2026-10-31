//! Decoders for USB Power Delivery Discover Identity data, written from the USB-PD
//! R3.x tables for the ID Header, Product and Passive/Active Cable VDOs.
use crate::model::{CableInfo, CableKind, CableSpeed};
use crate::vendors;

fn bits(v: u32, hi: u32, lo: u32) -> u32 {
    (v >> lo) & ((1u32 << (hi - lo + 1)) - 1)
}

pub fn vid(id_header: u32) -> u16 {
    bits(id_header, 15, 0) as u16
}

pub fn pid(product: u32) -> u16 {
    bits(product, 31, 16) as u16
}

/// Bits 29:27 of an SOP' ID Header.
pub fn cable_kind(id_header: u32) -> CableKind {
    match bits(id_header, 29, 27) {
        0b011 => CableKind::Passive,
        0b100 => CableKind::Active,
        0b110 => CableKind::Vpd,
        _ => CableKind::Unknown,
    }
}

/// Bits 29:27 of an SOP ID Header, as words a person would use.
pub fn partner_kind(id_header: u32) -> Option<&'static str> {
    match bits(id_header, 29, 27) {
        0b001 => Some("USB hub"),
        0b010 => Some("USB peripheral"),
        0b011 => Some("Power-only device"),
        _ => None,
    }
}

fn speed(vdo: u32) -> CableSpeed {
    match bits(vdo, 2, 0) {
        0b000 => CableSpeed::Usb2,
        0b001 => CableSpeed::Usb5g,
        0b010 => CableSpeed::Usb20g,
        0b011 => CableSpeed::Usb40g,
        0b100 => CableSpeed::Usb80g,
        _ => CableSpeed::Reserved,
    }
}

fn current_a(vdo: u32) -> Option<u8> {
    match bits(vdo, 6, 5) {
        0b01 => Some(3),
        0b10 => Some(5),
        _ => None,
    }
}

/// 20 V is the Standard Power Range ceiling; only an EPR 5 A cable carries 48 V.
fn max_watts(current: Option<u8>, epr: bool) -> Option<u32> {
    match (current?, epr) {
        (5, true) => Some(240),
        (a, _) => Some(a as u32 * 20),
    }
}

pub fn decode_cable(id_header: u32, cert_stat: u32, product: u32, vdo1: u32) -> CableInfo {
    let vendor_id = vid(id_header);
    let current = current_a(vdo1);
    let epr = bits(vdo1, 17, 17) == 1;
    CableInfo {
        kind: cable_kind(id_header),
        vendor_id,
        vendor_name: vendors::name(vendor_id).map(str::to_string),
        product_id: pid(product),
        xid: cert_stat,
        speed: speed(vdo1),
        current_a: current,
        epr,
        max_watts: max_watts(current, epr),
        raw_vdos: vec![id_header, cert_stat, product, vdo1],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Passive, 5 A, 40 Gbps, EPR, Type-C plug, VID 0x05AC.
    const HEADER: u32 = (0b011 << 27) | 0x05AC;
    const VDO: u32 = (0b10 << 18) | (1 << 17) | (0b10 << 5) | 0b011;

    #[test]
    fn decodes_full_spec_cable() {
        let c = decode_cable(HEADER, 0x1234, 0x7001_0100, VDO);
        assert_eq!(c.kind, CableKind::Passive);
        assert_eq!(c.vendor_id, 0x05AC);
        assert_eq!(c.product_id, 0x7001);
        assert_eq!(c.xid, 0x1234);
        assert_eq!(c.speed, CableSpeed::Usb40g);
        assert_eq!(c.current_a, Some(5));
        assert!(c.epr);
        assert_eq!(c.max_watts, Some(240));
    }

    #[test]
    fn watt_limits_follow_current_and_epr() {
        assert_eq!(max_watts(Some(3), false), Some(60));
        assert_eq!(max_watts(Some(5), false), Some(100));
        assert_eq!(max_watts(Some(5), true), Some(240));
        // EPR needs 5 A, so a 3 A cable claiming it stays at 60 W.
        assert_eq!(max_watts(Some(3), true), Some(60));
        assert_eq!(max_watts(None, false), None);
    }

    #[test]
    fn reserved_fields_stay_visible() {
        let c = decode_cable(HEADER, 0, 0, (0b11 << 5) | 0b111);
        assert_eq!(c.speed, CableSpeed::Reserved);
        assert_eq!(c.current_a, None);
    }

    #[test]
    fn active_cable_header() {
        assert_eq!(cable_kind(0b100 << 27), CableKind::Active);
        assert_eq!(partner_kind(0b010 << 27), Some("USB peripheral"));
    }
}
