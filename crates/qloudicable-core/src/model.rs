use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Linux,
    Macos,
    Windows,
    Android,
    #[default]
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Full,
    Partial,
    #[default]
    None,
}

/// What this OS lets us see for one class of data. The UI shows this so a blank
/// never gets mistaken for a bad cable.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Access {
    pub level: Level,
    pub why: String,
}

impl Access {
    pub fn new(level: Level, why: &str) -> Self {
        Self {
            level,
            why: why.to_string(),
        }
    }
    pub fn full() -> Self {
        Self::new(Level::Full, "")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Capabilities {
    pub cable_identity: Access,
    pub charger_profiles: Access,
    pub link_speed: Access,
    pub power_measurement: Access,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Snapshot {
    pub tool_version: String,
    pub platform: Platform,
    pub capabilities: Capabilities,
    pub ports: Vec<Port>,
    pub usb_devices: Vec<UsbDevice>,
    pub power: Option<PowerReading>,
    pub notes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Port {
    pub id: String,
    pub label: String,
    pub connected: bool,
    pub data_role: Option<String>,
    pub power_role: Option<String>,
    pub orientation: Option<String>,
    /// e.g. "usb_power_delivery", "3.0A", "1.5A", "default".
    pub power_mode: Option<String>,
    pub pd_revision: Option<String>,
    pub cable: Option<CableInfo>,
    pub partner: Option<Partner>,
    pub source_pdos: Vec<Pdo>,
    pub active_pdo: Option<Pdo>,
    pub transports: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CableKind {
    Passive,
    Active,
    Vpd,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CableSpeed {
    Usb2,
    Usb5g,
    Usb20g,
    Usb40g,
    Usb80g,
    Reserved,
}

impl CableSpeed {
    pub fn mbps(self) -> Option<u32> {
        match self {
            Self::Usb2 => Some(480),
            Self::Usb5g => Some(5_000),
            Self::Usb20g => Some(20_000),
            Self::Usb40g => Some(40_000),
            Self::Usb80g => Some(80_000),
            Self::Reserved => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Usb2 => "USB 2.0 (480 Mbps)",
            Self::Usb5g => "5 Gbps",
            Self::Usb20g => "10–20 Gbps",
            Self::Usb40g => "40 Gbps",
            Self::Usb80g => "80 Gbps",
            Self::Reserved => "not stated (reserved value)",
        }
    }
}

/// Decoded from the e-marker chip in the cable's plug.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CableInfo {
    pub kind: CableKind,
    pub vendor_id: u16,
    pub vendor_name: Option<String>,
    pub product_id: u16,
    pub xid: u32,
    pub speed: CableSpeed,
    /// Amps the e-marker states; None when the field holds a reserved value.
    pub current_a: Option<u8>,
    pub epr: bool,
    pub max_watts: Option<u32>,
    pub raw_vdos: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Partner {
    pub vendor_id: Option<u16>,
    pub vendor_name: Option<String>,
    pub product_id: Option<u16>,
    pub kind: Option<String>,
    pub supports_pd: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PdoKind {
    Fixed,
    Variable,
    Battery,
    Pps,
}

/// One power profile a charger offers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pdo {
    pub kind: PdoKind,
    pub min_mv: u32,
    pub max_mv: u32,
    pub max_ma: u32,
    pub max_mw: u32,
}

impl Pdo {
    pub fn fixed(mv: u32, ma: u32) -> Self {
        Self {
            kind: PdoKind::Fixed,
            min_mv: mv,
            max_mv: mv,
            max_ma: ma,
            max_mw: mv * ma / 1000,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct UsbDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub vendor_name: Option<String>,
    pub product_name: Option<String>,
    /// bcdUSB rendered as "3.20".
    pub usb_version: Option<String>,
    /// Speed the link is running at right now.
    pub speed_mbps: Option<u32>,
    /// Fastest speed the device itself is known to support.
    pub capable_mbps: Option<u32>,
    pub port_id: Option<String>,
    pub is_hub: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChargeState {
    Charging,
    Discharging,
    Full,
    NotCharging,
    #[default]
    Unknown,
}

/// Power measured at the battery or the input, not what was negotiated.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct PowerReading {
    pub state: ChargeState,
    pub external_power: Option<bool>,
    pub voltage_mv: Option<u32>,
    pub current_ma: Option<u32>,
    pub milliwatts: Option<u32>,
    pub battery_percent: Option<u8>,
    pub source: String,
}

impl PowerReading {
    /// One charge test sample. Battery drain is not charging power, so being on
    /// battery reads as zero, which the test counts as a dropout.
    pub fn charging_milliwatts(&self) -> Option<u32> {
        if self.state == ChargeState::Discharging || self.external_power == Some(false) {
            Some(0)
        } else {
            self.milliwatts
        }
    }
}

/// Link speed in words. Low speed (1.5 Mbps) is stored as 2 to keep the field whole.
pub fn speed_words(mbps: u32) -> String {
    match mbps {
        2 => "1.5 Mbps".to_string(),
        m if m >= 1000 => format!("{} Gbps", m / 1000),
        m => format!("{m} Mbps"),
    }
}
