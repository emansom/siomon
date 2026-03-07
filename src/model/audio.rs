use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub card_index: u32,
    pub card_id: String,
    pub card_long_name: String,
    pub driver: String,
    pub bus_type: AudioBusType,
    pub codec: Option<String>,
    pub pci_bus_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioBusType {
    HdAudio,
    Ac97,
    Usb,
    Virtual,
    Unknown(String),
}
