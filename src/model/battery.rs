use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub name: String,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
    pub chemistry: BatteryChemistry,
    pub status: BatteryStatus,
    pub design_capacity_uwh: Option<u64>,
    pub full_charge_capacity_uwh: Option<u64>,
    pub remaining_capacity_uwh: Option<u64>,
    pub voltage_now_uv: Option<u64>,
    pub power_now_uw: Option<u64>,
    pub capacity_percent: Option<u8>,
    pub cycle_count: Option<u32>,
    pub wear_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatteryChemistry {
    LithiumIon,
    LithiumPolymer,
    NickelMetalHydride,
    NickelCadmium,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}
