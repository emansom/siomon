use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDevice {
    pub bus: u8,
    pub port_path: String,
    pub devnum: u16,
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub usb_version: Option<String>,
    pub device_class: u8,
    pub speed: UsbSpeed,
    pub max_power_ma: Option<u32>,
    pub sysfs_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsbSpeed {
    Low,
    Full,
    High,
    Super,
    SuperPlus,
    SuperPlus2x2,
    Unknown(String),
}
