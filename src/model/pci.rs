use serde::{Deserialize, Serialize};

use super::gpu::PcieLinkInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PciDevice {
    pub address: String,
    pub domain: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub subsystem_vendor_id: Option<u16>,
    pub subsystem_device_id: Option<u16>,
    pub revision: u8,
    pub class_code: u32,
    pub vendor_name: Option<String>,
    pub device_name: Option<String>,
    pub class_name: Option<String>,
    pub subclass_name: Option<String>,
    pub driver: Option<String>,
    pub irq: Option<u32>,
    pub numa_node: Option<i32>,
    pub pcie_link: Option<PcieLinkInfo>,
    pub enabled: bool,
}
