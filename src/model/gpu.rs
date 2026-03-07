use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub index: u32,
    pub vendor: GpuVendor,
    pub name: String,
    pub architecture: Option<String>,
    pub pci_vendor_id: u16,
    pub pci_device_id: u16,
    pub pci_subsystem_vendor_id: Option<u16>,
    pub pci_subsystem_device_id: Option<u16>,
    pub pci_bus_address: String,
    pub drm_card_index: Option<u32>,
    pub vbios_version: Option<String>,
    pub driver_version: Option<String>,
    pub driver_module: Option<String>,
    pub vram_total_bytes: Option<u64>,
    pub vram_type: Option<String>,
    pub vram_bus_width_bits: Option<u32>,
    pub max_core_clock_mhz: Option<u32>,
    pub max_memory_clock_mhz: Option<u32>,
    pub compute_capability: Option<String>,
    pub shader_units: Option<u32>,
    pub power_limit_watts: Option<f64>,
    pub ecc_enabled: Option<bool>,
    pub pcie_link: Option<PcieLinkInfo>,
    pub display_outputs: Vec<DisplayOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcieLinkInfo {
    pub current_gen: Option<u8>,
    pub current_width: Option<u8>,
    pub max_gen: Option<u8>,
    pub max_width: Option<u8>,
    pub current_speed: Option<String>,
    pub max_speed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayOutput {
    pub connector_type: String,
    pub index: u32,
    pub status: String,
    pub monitor_name: Option<String>,
    pub resolution: Option<String>,
}
