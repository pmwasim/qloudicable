//! USB vendor names from the bundled usb.ids snapshot (see THIRD_PARTY_NOTICES.md).
static TABLE: &str = include_str!("../data/vendors.tsv");

/// The table is sorted by id, one `xxxx<TAB>name` line per vendor.
pub fn name(id: u16) -> Option<&'static str> {
    let want = format!("{id:04x}");
    TABLE.lines().find_map(|l| {
        l.split_once('\t')
            .filter(|(k, _)| *k == want)
            .map(|(_, v)| v)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn known_and_unknown_ids() {
        assert_eq!(super::name(0x05ac), Some("Apple, Inc."));
        assert_eq!(super::name(0x0000), None);
    }
}
