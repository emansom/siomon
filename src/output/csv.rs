//! CSV sensor logger.
//!
//! Writes sensor readings to a CSV file, one row per poll cycle. The column
//! order is established on the first call to [`CsvLogger::write_row`] by
//! snapshotting the current sensor keys and sorting them lexicographically.

#[cfg(feature = "csv")]
mod inner {
    use std::collections::HashMap;
    use std::path::Path;
    use std::sync::{Arc, RwLock};

    use chrono::Utc;

    use crate::model::sensor::{SensorId, SensorReading};

    pub struct CsvLogger {
        writer: csv::Writer<std::fs::File>,
        columns: Vec<SensorId>,
        initialized: bool,
    }

    impl CsvLogger {
        /// Create a new CSV logger that writes to the given file path.
        ///
        /// The file is created (or truncated) immediately. The header row is
        /// written on the first call to [`write_row`].
        pub fn new(path: &Path) -> std::io::Result<Self> {
            let file = std::fs::File::create(path)?;
            let writer = csv::Writer::from_writer(file);
            Ok(Self {
                writer,
                columns: Vec::new(),
                initialized: false,
            })
        }

        /// Write a row of sensor data.
        ///
        /// On the first call, discovers the current set of sensor keys, sorts
        /// them, and writes a header row (`"timestamp"`, then one column per
        /// sensor label). Every call writes the current timestamp and the
        /// current value of each sensor in column order. Sensors that appeared
        /// after the header was written are silently ignored; sensors that have
        /// disappeared produce an empty cell.
        pub fn write_row(
            &mut self,
            state: &Arc<RwLock<HashMap<SensorId, SensorReading>>>,
        ) -> std::io::Result<()> {
            let map = state.read().unwrap_or_else(|e| e.into_inner());

            if !self.initialized {
                self.initialized = true;

                // Snapshot and sort keys
                let mut keys: Vec<SensorId> = map.keys().cloned().collect();
                keys.sort_by(|a, b| {
                    a.source
                        .cmp(&b.source)
                        .then_with(|| a.chip.cmp(&b.chip))
                        .then_with(|| a.sensor.cmp(&b.sensor))
                });
                self.columns = keys;

                // Write header row
                let mut header: Vec<String> = Vec::with_capacity(self.columns.len() + 1);
                header.push("timestamp".to_string());
                for id in &self.columns {
                    let label = map
                        .get(id)
                        .map(|r| format!("{} [{}]", r.label, r.unit))
                        .unwrap_or_else(|| id.to_string());
                    header.push(label);
                }
                self.writer
                    .write_record(&header)
                    .map_err(std::io::Error::other)?;
            }

            // Write data row
            let now = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let mut row: Vec<String> = Vec::with_capacity(self.columns.len() + 1);
            row.push(now);
            for id in &self.columns {
                match map.get(id) {
                    Some(reading) => row.push(format!("{:.3}", reading.current)),
                    None => row.push(String::new()),
                }
            }
            drop(map); // Release the lock before flushing

            self.writer
                .write_record(&row)
                .map_err(std::io::Error::other)?;
            self.writer.flush()?;

            Ok(())
        }
    }
}

#[cfg(feature = "csv")]
pub use inner::CsvLogger;
