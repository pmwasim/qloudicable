//! Windows: the USB hub IOCTLs and the battery driver. Windows has no public way
//! to read a cable's e-marker or the charger's PD profiles from an app, so those
//! are declared unavailable, while "device can do X, link runs at Y" stays exact.
use qloudicable_core::vendors;
use qloudicable_core::*;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::*;
use windows_sys::Win32::Devices::Usb::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Power::*;
use windows_sys::core::GUID;

const GUID_USB_HUB: GUID = GUID::from_u128(0xf18a0e88_c30c_11d0_8815_00a0c906bed8);
const GUID_BATTERY: GUID = GUID::from_u128(0x72631e54_78a4_11d0_bcf7_00aa00b7b32a);
const IOCTL_BATTERY_QUERY_TAG: u32 = 0x294040;
const IOCTL_BATTERY_QUERY_STATUS: u32 = 0x29404c;

struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

/// Device paths for every interface of a class.
fn interface_paths(guid: &GUID) -> Vec<Vec<u16>> {
    let mut out = Vec::new();
    unsafe {
        let set = SetupDiGetClassDevsW(
            guid,
            null(),
            null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );
        if set as isize == -1 {
            return out;
        }
        let mut index = 0;
        loop {
            let mut iface: SP_DEVICE_INTERFACE_DATA = zeroed();
            iface.cbSize = size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
            if SetupDiEnumDeviceInterfaces(set, null(), guid, index, &mut iface) == 0 {
                break;
            }
            index += 1;
            let mut needed = 0u32;
            SetupDiGetDeviceInterfaceDetailW(set, &iface, null_mut(), 0, &mut needed, null_mut());
            if needed == 0 {
                continue;
            }
            // Over-aligned buffer for the header, which is followed by the path.
            let mut buf = vec![0u64; needed as usize / 8 + 1];
            let detail = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            if SetupDiGetDeviceInterfaceDetailW(set, &iface, detail, needed, null_mut(), null_mut())
                != 0
            {
                let start = std::ptr::addr_of!((*detail).DevicePath) as *const u16;
                let mut path = Vec::new();
                let mut i = 0;
                while *start.add(i) != 0 {
                    path.push(*start.add(i));
                    i += 1;
                }
                path.push(0);
                out.push(path);
            }
        }
        SetupDiDestroyDeviceInfoList(set);
    }
    out
}

fn open(path: &[u16], write: bool) -> Option<Handle> {
    let access = if write {
        GENERIC_READ | GENERIC_WRITE
    } else {
        GENERIC_READ
    };
    let h = unsafe {
        CreateFileW(
            path.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };
    (h != INVALID_HANDLE_VALUE).then(|| Handle(h))
}

/// Same buffer in and out, as the USB hub IOCTLs expect.
unsafe fn ioctl_inout<T>(h: &Handle, code: u32, data: *mut T) -> bool {
    let mut got = 0u32;
    let size = size_of::<T>() as u32;
    unsafe {
        DeviceIoControl(
            h.0,
            code,
            data as _,
            size,
            data as _,
            size,
            &mut got,
            null_mut(),
        ) != 0
    }
}

fn operating_mbps(speed: u8, v2_flags: u32) -> u32 {
    // Speed enum: 0 low, 1 full, 2 high, 3 super. The V2 flag separates 5 Gbps
    // from SuperSpeedPlus, which is at least 10 Gbps.
    const OPERATING_SSP: u32 = 1 << 2;
    match speed {
        0 => 2,
        1 => 12,
        2 => 480,
        _ if v2_flags & OPERATING_SSP != 0 => 10_000,
        _ => 5_000,
    }
}

fn capable_mbps(v2_flags: u32) -> Option<u32> {
    const SS_CAPABLE: u32 = 1 << 1;
    const SSP_CAPABLE: u32 = 1 << 3;
    if v2_flags & SSP_CAPABLE != 0 {
        Some(10_000)
    } else if v2_flags & SS_CAPABLE != 0 {
        Some(5_000)
    } else {
        None
    }
}

fn read_hub(path: &[u16], hub_index: usize, ports: &mut Vec<Port>, devices: &mut Vec<UsbDevice>) {
    let Some(hub) = open(path, true) else { return };
    unsafe {
        let mut node: USB_NODE_INFORMATION = zeroed();
        if !ioctl_inout(&hub, IOCTL_USB_GET_NODE_INFORMATION, &mut node) {
            return;
        }
        let count = node.u.HubInformation.HubDescriptor.bNumberOfPorts as u32;
        for port in 1..=count {
            let mut conn: USB_NODE_CONNECTION_INFORMATION_EX = zeroed();
            conn.ConnectionIndex = port;
            if !ioctl_inout(
                &hub,
                IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX,
                &mut conn,
            ) {
                continue;
            }
            let mut props: USB_PORT_CONNECTOR_PROPERTIES = zeroed();
            props.ConnectionIndex = port;
            let has_props = ioctl_inout(&hub, IOCTL_USB_GET_PORT_CONNECTOR_PROPERTIES, &mut props);
            // Bit 3 of UsbPortProperties is PortConnectorIsTypeC.
            let is_type_c = has_props && props.UsbPortProperties.ul & (1 << 3) != 0;
            let connected = conn.ConnectionStatus == DeviceConnected;
            let id = format!("hub{hub_index}-{port}");
            if is_type_c {
                ports.push(Port {
                    id: id.clone(),
                    label: format!("USB-C port (hub {hub_index}, port {port})"),
                    connected,
                    ..Default::default()
                });
            }
            if !connected {
                continue;
            }
            let mut v2: USB_NODE_CONNECTION_INFORMATION_EX_V2 = zeroed();
            v2.ConnectionIndex = port;
            v2.Length = size_of::<USB_NODE_CONNECTION_INFORMATION_EX_V2>() as u32;
            v2.SupportedUsbProtocols.ul = 0b111; // USB 1.1, 2.0 and 3.0
            let flags = if ioctl_inout(
                &hub,
                IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX_V2,
                &mut v2,
            ) {
                v2.Flags.ul
            } else {
                0
            };
            let d = &conn.DeviceDescriptor;
            let vendor_id = d.idVendor;
            let bcd = d.bcdUSB;
            devices.push(UsbDevice {
                vendor_id,
                product_id: d.idProduct,
                vendor_name: vendors::name(vendor_id).map(str::to_string),
                product_name: None,
                usb_version: Some(format!("{:x}.{:02x}", bcd >> 8, bcd & 0xFF)),
                speed_mbps: Some(operating_mbps(conn.Speed, flags)),
                capable_mbps: capable_mbps(flags),
                port_id: is_type_c.then_some(id),
                is_hub: conn.DeviceIsHub != 0,
            });
        }
    }
}

fn battery() -> Option<PowerReading> {
    let path = interface_paths(&GUID_BATTERY).into_iter().next()?;
    let h = open(&path, true)?;
    unsafe {
        let mut wait = 0u32;
        let mut tag = 0u32;
        let mut got = 0u32;
        if DeviceIoControl(
            h.0,
            IOCTL_BATTERY_QUERY_TAG,
            &mut wait as *mut u32 as _,
            4,
            &mut tag as *mut u32 as _,
            4,
            &mut got,
            null_mut(),
        ) == 0
            || tag == 0
        {
            return None;
        }
        let mut q: BATTERY_WAIT_STATUS = zeroed();
        q.BatteryTag = tag;
        let mut st: BATTERY_STATUS = zeroed();
        if DeviceIoControl(
            h.0,
            IOCTL_BATTERY_QUERY_STATUS,
            &mut q as *mut _ as _,
            size_of::<BATTERY_WAIT_STATUS>() as u32,
            &mut st as *mut _ as _,
            size_of::<BATTERY_STATUS>() as u32,
            &mut got,
            null_mut(),
        ) == 0
        {
            return None;
        }
        const ONLINE: u32 = 1;
        const CHARGING: u32 = 4;
        const DISCHARGING: u32 = 2;
        const UNKNOWN: u32 = 0xFFFF_FFFF;
        let known = |v: u32| (v != UNKNOWN).then_some(v);
        let external = st.PowerState & ONLINE != 0;
        let voltage_mv = known(st.Voltage);
        let rate = (st.Rate != 0x8000_0000u32 as i32).then(|| st.Rate.unsigned_abs());
        Some(PowerReading {
            state: if st.PowerState & CHARGING != 0 {
                ChargeState::Charging
            } else if st.PowerState & DISCHARGING != 0 {
                ChargeState::Discharging
            } else if external {
                ChargeState::NotCharging
            } else {
                ChargeState::Unknown
            },
            external_power: Some(external),
            voltage_mv,
            current_ma: voltage_mv
                .zip(rate)
                .filter(|(v, _)| *v > 0)
                .map(|(v, r)| r * 1000 / v),
            milliwatts: rate,
            battery_percent: None,
            source: "Windows battery driver".into(),
        })
    }
}

pub fn collect() -> Snapshot {
    let mut ports = Vec::new();
    let mut usb_devices = Vec::new();
    for (i, path) in interface_paths(&GUID_USB_HUB).iter().enumerate() {
        read_hub(path, i, &mut ports, &mut usb_devices);
    }
    let power = battery();
    let hidden =
        "Windows does not let apps read this. The USB-C driver keeps it inside the kernel.";
    Snapshot {
        platform: Platform::Windows,
        capabilities: Capabilities {
            cable_identity: Access::new(Level::None, hidden),
            charger_profiles: Access::new(Level::None, hidden),
            link_speed: Access::full(),
            power_measurement: if power.as_ref().is_some_and(|p| p.milliwatts.is_some()) {
                Access::new(
                    Level::Partial,
                    "Measured at the battery, so it reads below the charger's rating.",
                )
            } else {
                Access::new(Level::None, "No battery reading (desktop PC?).")
            },
        },
        ports,
        usb_devices,
        power,
        ..Default::default()
    }
}
