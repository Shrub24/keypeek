//! Keyboard device discovery.
//!
//! Scans for devices using registered [`DeviceDriverScanner`] implementations,
//! sharing transport snapshots (for example USB HID devices) across scanners.

use crate::protocols::ConnectionSpec;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub base_name: String,
    pub vid: u16,
    pub pid: u16,
    pub driver_id: &'static str,
    pub protocol_label: &'static str,
    pub requires_layout_file: bool,
    pub spec: ConnectionSpec,
}

impl DiscoveredDevice {
    pub fn display_name(&self) -> String {
        format!(
            "{} ({}, {:04X}:{:04X})",
            self.base_name, self.protocol_label, self.vid, self.pid
        )
    }

    /// User-friendly message explaining that the keyboard is locked and how to unlock it.
    pub fn lock_message(&self) -> String {
        format!(
            "Device '{}' is locked. Please unlock it on the keyboard (e.g. with its unlock key combination) and try again.",
            self.base_name
        )
    }

    /// Creates a generic fallback device descriptor when device details are unknown.
    pub fn anonymous() -> Self {
        Self {
            base_name: "Keyboard".to_string(),
            vid: 0,
            pid: 0,
            driver_id: "unknown",
            protocol_label: "unknown",
            requires_layout_file: false,
            spec: ConnectionSpec::Vial { vid: 0, pid: 0 },
        }
    }
}

/// Stable identity of a keyboard model, used to recognise it in a later session.
///
/// Transport details are deliberately absent: a serial path or BLE address
/// identifies a connection, not a keyboard, and changes between reboots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub driver_id: String,
    pub vid: u16,
    pub pid: u16,
    pub name: String,
}

impl DeviceIdentity {
    pub fn from_device(device: &DiscoveredDevice) -> Self {
        Self {
            driver_id: device.driver_id.to_string(),
            vid: device.vid,
            pid: device.pid,
            name: device.base_name.clone(),
        }
    }

    pub fn matches(&self, device: &DiscoveredDevice) -> bool {
        self.driver_id == device.driver_id
            && self.vid == device.vid
            && self.pid == device.pid
            && self.name == device.base_name
    }

    /// Single-line form written to the session state file: `driver:vid:pid:name`.
    pub fn to_wire(&self) -> String {
        format!(
            "{}:{:04x}:{:04x}:{}",
            self.driver_id, self.vid, self.pid, self.name
        )
    }

    /// Parses [`Self::to_wire`]; the name may itself contain colons.
    pub fn parse(value: &str) -> Option<Self> {
        let mut parts = value.trim().splitn(4, ':');
        let driver_id = parts.next()?.to_string();
        let vid = u16::from_str_radix(parts.next()?, 16).ok()?;
        let pid = u16::from_str_radix(parts.next()?, 16).ok()?;
        let name = parts.next()?.to_string();
        if driver_id.is_empty() || name.is_empty() {
            return None;
        }
        Some(Self {
            driver_id,
            vid,
            pid,
            name,
        })
    }
}

/// Generic, protocol-agnostic snapshot of an attached USB HID interface.
#[derive(Clone, Debug)]
pub struct HidDeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub usage_page: u16,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}

/// Context shared across device driver scanners during discovery.
#[derive(Default)]
pub struct DiscoveryContext {
    hid_devices: Vec<HidDeviceInfo>,
    claimed_vid_pids: HashSet<(u16, u16)>,
}

impl DiscoveryContext {
    pub fn new(hid_devices: Vec<HidDeviceInfo>) -> Self {
        Self {
            hid_devices,
            claimed_vid_pids: HashSet::new(),
        }
    }

    pub fn hid_devices(&self) -> &[HidDeviceInfo] {
        &self.hid_devices
    }

    pub fn is_claimed(&self, vid: u16, pid: u16) -> bool {
        self.claimed_vid_pids.contains(&(vid, pid))
    }

    pub fn claim(&mut self, vid: u16, pid: u16) {
        self.claimed_vid_pids.insert((vid, pid));
    }

    /// Finds the product name of an HID device matching the given VID/PID, if present.
    pub fn find_product(&self, vid: u16, pid: u16) -> Option<String> {
        self.hid_devices
            .iter()
            .find(|d| d.vendor_id == vid && d.product_id == pid)
            .and_then(|d| d.product.clone())
    }
}

/// Port for firmware adapters to discover compatible hardware.
pub trait DeviceDriverScanner: Send + Sync {
    fn scan(&self, ctx: &mut DiscoveryContext) -> Vec<DiscoveredDevice>;
}

/// Returns the standard list of registered device scanners from all firmware bundles.
pub fn default_scanners() -> Vec<Box<dyn DeviceDriverScanner>> {
    crate::firmware::default_scanners()
}

/// Runs discovery over the specified scanners and context.
pub fn discover_devices_with(
    ctx: &mut DiscoveryContext,
    scanners: &[Box<dyn DeviceDriverScanner>],
) -> Vec<DiscoveredDevice> {
    let mut devices = Vec::new();
    for scanner in scanners {
        devices.extend(scanner.scan(ctx));
    }

    devices.sort_by_cached_key(|d| d.display_name());
    devices.dedup_by(|a, b| {
        a.vid == b.vid && a.pid == b.pid && a.driver_id == b.driver_id && a.spec == b.spec
    });

    devices
}

/// Discovers all available keyboards using provided HID interfaces and default protocol scanners.
pub fn discover_devices(hid_devices: Vec<HidDeviceInfo>) -> Vec<DiscoveredDevice> {
    let mut ctx = DiscoveryContext::new(hid_devices);
    let scanners = default_scanners();
    discover_devices_with(&mut ctx, &scanners)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_spec() -> ConnectionSpec {
        crate::test_utils::test_spec()
    }

    #[test]
    fn identity_round_trips_through_the_wire_form() {
        let corne = DiscoveredDevice {
            base_name: "Corne".to_string(),
            vid: 0x1d50,
            pid: 0x615e,
            driver_id: "zmk",
            protocol_label: "ZMK Serial",
            requires_layout_file: false,
            spec: dummy_spec(),
        };
        let identity = DeviceIdentity::from_device(&corne);
        assert_eq!(identity.to_wire(), "zmk:1d50:615e:Corne");
        assert_eq!(DeviceIdentity::parse(&identity.to_wire()), Some(identity));
    }

    #[test]
    fn identity_keeps_colons_in_the_name() {
        let identity = DeviceIdentity {
            driver_id: "zmk".to_string(),
            vid: 0x1d50,
            pid: 0x615e,
            name: "Split: Corne".to_string(),
        };
        assert_eq!(DeviceIdentity::parse(&identity.to_wire()), Some(identity));
    }

    #[test]
    fn identity_rejects_malformed_values() {
        assert_eq!(DeviceIdentity::parse(""), None);
        assert_eq!(DeviceIdentity::parse("zmk:zz:615e:Corne"), None);
        assert_eq!(DeviceIdentity::parse("zmk:1d50:615e"), None);
        assert_eq!(DeviceIdentity::parse(":1d50:615e:Corne"), None);
    }

    #[test]
    fn identity_matches_the_same_board_on_another_transport() {
        let corne = DiscoveredDevice {
            base_name: "Corne".to_string(),
            vid: 0x1d50,
            pid: 0x615e,
            driver_id: "zmk",
            protocol_label: "ZMK Serial",
            requires_layout_file: false,
            spec: dummy_spec(),
        };
        let identity = DeviceIdentity::from_device(&corne);

        let over_ble = DiscoveredDevice {
            protocol_label: "ZMK BLE",
            ..corne.clone()
        };
        assert!(identity.matches(&over_ble));

        let other_firmware = DiscoveredDevice {
            driver_id: "vial",
            protocol_label: "Vial",
            ..corne.clone()
        };
        assert!(!identity.matches(&other_firmware));

        let other_board = DiscoveredDevice {
            base_name: "Lily58".to_string(),
            ..corne
        };
        assert!(!identity.matches(&other_board));
    }

    #[test]
    fn display_name_uses_protocol_label() {
        let board = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 0x1234,
            pid: 0xABCD,
            driver_id: "zmk",
            protocol_label: "ZMK BLE",
            requires_layout_file: false,
            spec: dummy_spec(),
        };
        assert_eq!(board.display_name(), "Board (ZMK BLE, 1234:ABCD)");
    }

    #[test]
    fn display_name_for_various_protocols() {
        let vial_board = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 0,
            pid: 0,
            driver_id: "vial",
            protocol_label: "Vial",
            requires_layout_file: false,
            spec: dummy_spec(),
        };
        let qmk_board = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 0x0A0B,
            pid: 0x0C0D,
            driver_id: "via",
            protocol_label: "QMK",
            requires_layout_file: true,
            spec: dummy_spec(),
        };
        assert_eq!(vial_board.display_name(), "Board (Vial, 0000:0000)");
        assert_eq!(qmk_board.display_name(), "Board (QMK, 0A0B:0C0D)");
    }

    #[test]
    fn zmk_transport_label_variants() {
        let serial = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 1,
            pid: 2,
            driver_id: "zmk",
            protocol_label: "ZMK Serial",
            requires_layout_file: false,
            spec: dummy_spec(),
        };
        let ble = DiscoveredDevice {
            base_name: "Board".to_string(),
            vid: 1,
            pid: 2,
            driver_id: "zmk",
            protocol_label: "ZMK BLE",
            requires_layout_file: false,
            spec: dummy_spec(),
        };
        assert!(serial.display_name().contains("ZMK Serial"));
        assert!(ble.display_name().contains("ZMK BLE"));
    }

    struct TestScanner {
        devices: Vec<DiscoveredDevice>,
    }

    impl DeviceDriverScanner for TestScanner {
        fn scan(&self, ctx: &mut DiscoveryContext) -> Vec<DiscoveredDevice> {
            let mut result = Vec::new();
            for d in &self.devices {
                if !ctx.is_claimed(d.vid, d.pid) {
                    ctx.claim(d.vid, d.pid);
                    result.push(d.clone());
                }
            }
            result
        }
    }

    #[test]
    fn discover_devices_with_deduplicates_and_sorts() {
        let mut ctx = DiscoveryContext::default();
        let scanner1: Box<dyn DeviceDriverScanner> = Box::new(TestScanner {
            devices: vec![
                DiscoveredDevice {
                    base_name: "B Keyboard".to_string(),
                    vid: 0x0002,
                    pid: 0x0002,
                    driver_id: "via",
                    protocol_label: "QMK",
                    requires_layout_file: true,
                    spec: dummy_spec(),
                },
                DiscoveredDevice {
                    base_name: "A Keyboard".to_string(),
                    vid: 0x0001,
                    pid: 0x0001,
                    driver_id: "via",
                    protocol_label: "QMK",
                    requires_layout_file: true,
                    spec: dummy_spec(),
                },
            ],
        });

        let scanner2: Box<dyn DeviceDriverScanner> = Box::new(TestScanner {
            devices: vec![
                // Already claimed VID/PID 0x0001:0x0001 by scanner1, should be skipped
                DiscoveredDevice {
                    base_name: "A Keyboard Duplicate".to_string(),
                    vid: 0x0001,
                    pid: 0x0001,
                    driver_id: "vial",
                    protocol_label: "Vial",
                    requires_layout_file: false,
                    spec: dummy_spec(),
                },
                DiscoveredDevice {
                    base_name: "C Keyboard".to_string(),
                    vid: 0x0003,
                    pid: 0x0003,
                    driver_id: "zmk",
                    protocol_label: "ZMK",
                    requires_layout_file: false,
                    spec: dummy_spec(),
                },
            ],
        });

        let scanners = vec![scanner1, scanner2];
        let discovered = discover_devices_with(&mut ctx, &scanners);

        assert_eq!(discovered.len(), 3);
        assert_eq!(discovered[0].base_name, "A Keyboard");
        assert_eq!(discovered[1].base_name, "B Keyboard");
        assert_eq!(discovered[2].base_name, "C Keyboard");
    }
}
