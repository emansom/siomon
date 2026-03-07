use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAdapter {
    pub name: String,
    pub driver: Option<String>,
    pub mac_address: Option<String>,
    pub permanent_mac: Option<String>,
    pub speed_mbps: Option<u32>,
    pub operstate: String,
    pub duplex: Option<String>,
    pub mtu: u32,
    pub interface_type: NetworkInterfaceType,
    pub is_physical: bool,
    pub pci_bus_address: Option<String>,
    pub pci_vendor_id: Option<u16>,
    pub pci_device_id: Option<u16>,
    pub ip_addresses: Vec<IpAddress>,
    pub numa_node: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkInterfaceType {
    Ethernet,
    Wifi,
    Bridge,
    Bond,
    Vlan,
    Loopback,
    Virtual,
    Tun,
    Unknown(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpAddress {
    pub address: String,
    pub prefix_len: u8,
    pub family: String,
    pub scope: Option<String>,
}
