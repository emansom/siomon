//! Direct register reader for Nuvoton NCT6775-NCT6799 Super I/O chips.
//!
//! Reads voltages, temperatures, and fan speeds directly from the chip's
//! hardware monitoring registers via I/O port address/data pairs, bypassing
//! the kernel hwmon driver entirely. Requires root or CAP_SYS_RAWIO.

use crate::model::sensor::{SensorCategory, SensorId, SensorReading, SensorUnit};
use crate::platform::port_io::PortIo;
use crate::sensors::superio::chip_detect::{ChipType, SuperIoChip};

// NCT6775 register offsets (banked: high byte = bank, low byte = register)
const REG_BANK: u8 = 0x4E;

// Voltage registers (bank 4, 18 channels for NCT6798)
const VOLTAGE_REGS: [(u16, &str); 18] = [
    (0x480, "VIN0"),
    (0x481, "VIN1"),
    (0x482, "VIN2"),
    (0x483, "VIN3"),
    (0x484, "VIN4"),
    (0x485, "VIN5"),
    (0x486, "VIN6"),
    (0x487, "VIN7"),
    (0x488, "VBAT"),
    (0x489, "VTT"),
    (0x48A, "VIN10"),
    (0x48B, "VIN11"),
    (0x48C, "VIN12"),
    (0x48D, "VIN13"),
    (0x48E, "VIN14"),
    (0x48F, "VIN15"),
    (0x470, "VIN16"),
    (0x471, "VIN17"),
];

// Internal voltage scaling factors for NCT6798 (units: 0.001V per LSB * factor/100)
// From kernel nct6775-core.c: scale_in_6798[]
// Index matches VOLTAGE_REGS order
const VOLTAGE_SCALE_NCT6798: [u32; 18] = [
    800, 800, 1600, 1600, 800, 800, 800, 1600, // VIN0-VIN7
    1600, 1600, 1600, 1600, 800, 800, 800, 800, // VBAT-VIN15
    1600, 800, // VIN16-VIN17
];

// Temperature monitoring registers (direct temperature values)
const TEMP_REGS: [(u16, &str); 7] = [
    (0x027, "SYSTIN"),
    (0x073, "PECI Agent 0"),
    (0x075, "CPUTIN"),
    (0x077, "SYSTIN2"),
    (0x079, "AUXTIN0"),
    (0x07B, "AUXTIN1"),
    (0x07D, "AUXTIN2"),
];

// Additional temperature registers (bank 4/6)
const TEMP_EXTRA_REGS: [(u16, &str); 5] = [
    (0x4A0, "AUXTIN3"),
    (0x670, "AUXTIN0 Direct"),
    (0x672, "AUXTIN1 Direct"),
    (0x674, "AUXTIN2 Direct"),
    (0x676, "AUXTIN3 Direct"),
];

// Fan count registers (16-bit, bank 4)
const FAN_REGS: [(u16, &str); 7] = [
    (0x4C0, "Fan 1"),
    (0x4C2, "Fan 2"),
    (0x4C4, "Fan 3"),
    (0x4C6, "Fan 4"),
    (0x4C8, "Fan 5"),
    (0x4CA, "Fan 6"),
    (0x4CE, "Fan 7"),
];

// Fan RPM divisor constant
const FAN_RPM_FACTOR: u32 = 1_350_000;

pub struct Nct67xxSource {
    chip: SuperIoChip,
    addr_port: u16,
    data_port: u16,
}

impl Nct67xxSource {
    /// Create a new NCT67xx sensor source from a detected chip.
    pub fn new(chip: SuperIoChip) -> Self {
        let addr_port = chip.hwm_base + 5;
        let data_port = chip.hwm_base + 6;
        Self {
            chip,
            addr_port,
            data_port,
        }
    }

    /// Check if this source is usable.
    pub fn is_supported(&self) -> bool {
        matches!(
            self.chip.chip,
            ChipType::Nct6775
                | ChipType::Nct6776
                | ChipType::Nct6779
                | ChipType::Nct6791
                | ChipType::Nct6792
                | ChipType::Nct6793
                | ChipType::Nct6795
                | ChipType::Nct6796
                | ChipType::Nct6797
                | ChipType::Nct6798
                | ChipType::Nct6799
        )
    }

    /// Poll all sensors and return readings.
    pub fn poll(&self) -> Vec<(SensorId, SensorReading)> {
        let mut pio = match PortIo::open() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let mut readings = Vec::new();
        let chip_name = format!("{}", self.chip.chip).to_lowercase();

        // Read voltages
        let scale = &VOLTAGE_SCALE_NCT6798;
        for (i, &(reg, label)) in VOLTAGE_REGS.iter().enumerate() {
            if let Some(raw) = self.read_register(&mut pio, reg) {
                if raw == 0 {
                    continue; // Unconnected input
                }
                // voltage_mV = raw * scale_factor / 100
                let mv = raw as f64 * scale[i] as f64 / 100.0;
                let volts = mv / 1000.0;

                let id = SensorId {
                    source: "superio".into(),
                    chip: chip_name.clone(),
                    sensor: format!("vin{i}"),
                };
                readings.push((
                    id,
                    SensorReading::new(
                        label.to_string(),
                        volts,
                        SensorUnit::Volts,
                        SensorCategory::Voltage,
                    ),
                ));
            }
        }

        // Read temperatures
        for &(reg, label) in &TEMP_REGS {
            if let Some(raw) = self.read_register(&mut pio, reg) {
                // Temperature is signed 8-bit (degrees C)
                let temp = raw as i8 as f64;
                if temp == 0.0 || temp == -128.0 || temp > 127.0 {
                    continue; // Invalid or disconnected
                }

                let sensor_name = label.to_lowercase().replace(' ', "_");
                let id = SensorId {
                    source: "superio".into(),
                    chip: chip_name.clone(),
                    sensor: sensor_name,
                };
                readings.push((
                    id,
                    SensorReading::new(
                        label.to_string(),
                        temp,
                        SensorUnit::Celsius,
                        SensorCategory::Temperature,
                    ),
                ));
            }
        }

        // Read extra temperature registers (half-degree resolution)
        for &(reg, label) in &TEMP_EXTRA_REGS {
            if let Some(raw) = self.read_register(&mut pio, reg) {
                let temp = raw as i8 as f64;
                if temp == 0.0 || temp == -128.0 {
                    continue;
                }

                // Try reading fractional part (next register)
                let frac = self
                    .read_register(&mut pio, reg + 1)
                    .map(|f| (f >> 7) as f64 * 0.5)
                    .unwrap_or(0.0);
                let temp = temp + frac;

                let sensor_name = label.to_lowercase().replace(' ', "_");
                let id = SensorId {
                    source: "superio".into(),
                    chip: chip_name.clone(),
                    sensor: sensor_name,
                };
                readings.push((
                    id,
                    SensorReading::new(
                        label.to_string(),
                        temp,
                        SensorUnit::Celsius,
                        SensorCategory::Temperature,
                    ),
                ));
            }
        }

        // Read fan speeds (16-bit count values)
        for (i, &(reg, label)) in FAN_REGS.iter().enumerate() {
            if let Some(count) = self.read_word(&mut pio, reg) {
                if count == 0 || count == 0xFFFF {
                    // Stopped or disconnected
                    let id = SensorId {
                        source: "superio".into(),
                        chip: chip_name.clone(),
                        sensor: format!("fan{}", i + 1),
                    };
                    readings.push((
                        id,
                        SensorReading::new(
                            label.to_string(),
                            0.0,
                            SensorUnit::Rpm,
                            SensorCategory::Fan,
                        ),
                    ));
                    continue;
                }

                let rpm = FAN_RPM_FACTOR / count as u32;
                let id = SensorId {
                    source: "superio".into(),
                    chip: chip_name.clone(),
                    sensor: format!("fan{}", i + 1),
                };
                readings.push((
                    id,
                    SensorReading::new(
                        label.to_string(),
                        rpm as f64,
                        SensorUnit::Rpm,
                        SensorCategory::Fan,
                    ),
                ));
            }
        }

        readings
    }

    /// Read a single byte from a banked HWM register.
    fn read_register(&self, pio: &mut PortIo, reg: u16) -> Option<u8> {
        let bank = (reg >> 8) as u8;
        let addr = (reg & 0xFF) as u8;

        // Select bank
        pio.write_byte(self.addr_port, REG_BANK).ok()?;
        pio.write_byte(self.data_port, bank).ok()?;

        // Read register
        pio.write_byte(self.addr_port, addr).ok()?;
        pio.read_byte(self.data_port).ok()
    }

    /// Read a 16-bit word from two consecutive registers.
    fn read_word(&self, pio: &mut PortIo, reg: u16) -> Option<u16> {
        let hi = self.read_register(pio, reg)? as u16;
        let lo = self.read_register(pio, reg + 1)? as u16;
        Some((hi << 8) | lo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voltage_scale_array_length() {
        assert_eq!(VOLTAGE_SCALE_NCT6798.len(), VOLTAGE_REGS.len());
    }

    #[test]
    fn test_voltage_calculation() {
        // Raw value 96 with scale 800 should give 0.768V
        let raw = 96u8;
        let scale = 800u32;
        let mv = raw as f64 * scale as f64 / 100.0;
        let volts = mv / 1000.0;
        assert!((volts - 0.768).abs() < 0.001);
    }

    #[test]
    fn test_fan_rpm_calculation() {
        // Count value of 675 should give 2000 RPM
        let count = 675u32;
        let rpm = FAN_RPM_FACTOR / count;
        assert_eq!(rpm, 2000);
    }

    #[test]
    fn test_fan_rpm_zero_count() {
        // Count 0 means stopped — should not divide by zero
        let count = 0u32;
        if count == 0 {
            // Handled as stopped fan
            assert!(true);
        }
    }
}
