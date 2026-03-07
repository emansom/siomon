use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use crate::model::sensor::{SensorId, SensorReading};
use crate::sensors::{cpu_freq, cpu_util, disk_activity, gpu_sensors, hwmon, network_stats, rapl};

pub type SensorState = Arc<RwLock<HashMap<SensorId, SensorReading>>>;

pub fn new_state() -> SensorState {
    Arc::new(RwLock::new(HashMap::new()))
}

pub struct Poller {
    state: SensorState,
    interval: Duration,
    no_nvidia: bool,
    label_overrides: HashMap<String, String>,
}

impl Poller {
    pub fn new(
        state: SensorState,
        interval_ms: u64,
        no_nvidia: bool,
        label_overrides: HashMap<String, String>,
    ) -> Self {
        Self {
            state,
            interval: Duration::from_millis(interval_ms),
            no_nvidia,
            label_overrides,
        }
    }

    /// Run the polling loop in a background thread. Returns a handle to stop it.
    pub fn spawn(self) -> PollerHandle {
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_clone = stop.clone();

        let handle = thread::spawn(move || {
            self.run(stop_clone);
        });

        PollerHandle {
            stop,
            _handle: handle,
        }
    }

    fn run(self, stop: Arc<std::sync::atomic::AtomicBool>) {
        // Discover all sensor sources
        let hwmon_src = hwmon::HwmonSource::discover(&self.label_overrides);
        let freq_src = cpu_freq::CpuFreqSource::discover();
        let mut util_src = cpu_util::CpuUtilSource::discover();
        let gpu_src = gpu_sensors::GpuSensorSource::discover(self.no_nvidia);
        let mut rapl_src = rapl::RaplSource::discover();
        let mut disk_src = disk_activity::DiskActivitySource::discover();
        let mut net_src = network_stats::NetworkStatsSource::discover();

        log::info!(
            "Sensor poller started: {} hwmon chips, {} hwmon sensors",
            hwmon_src.chip_count(),
            hwmon_src.sensor_count()
        );

        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
            let mut new_readings: Vec<(SensorId, SensorReading)> = Vec::new();

            // Collect from all sources
            new_readings.extend(hwmon_src.poll());
            new_readings.extend(freq_src.poll());
            new_readings.extend(util_src.poll());
            new_readings.extend(gpu_src.poll());
            new_readings.extend(rapl_src.poll());
            new_readings.extend(disk_src.poll());
            new_readings.extend(net_src.poll());

            // Update shared state
            if let Ok(mut state) = self.state.write() {
                for (id, new_reading) in new_readings {
                    if let Some(existing) = state.get_mut(&id) {
                        existing.update(new_reading.current);
                    } else {
                        state.insert(id, new_reading);
                    }
                }
            }

            thread::sleep(self.interval);
        }
    }
}

pub struct PollerHandle {
    stop: Arc<std::sync::atomic::AtomicBool>,
    _handle: thread::JoinHandle<()>,
}

impl PollerHandle {
    pub fn stop(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Drop for PollerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Take a one-shot snapshot of all sensors (single poll cycle).
pub fn snapshot(
    no_nvidia: bool,
    label_overrides: &HashMap<String, String>,
) -> HashMap<SensorId, SensorReading> {
    let hwmon_src = hwmon::HwmonSource::discover(label_overrides);
    let freq_src = cpu_freq::CpuFreqSource::discover();
    let mut util_src = cpu_util::CpuUtilSource::discover();
    let gpu_src = gpu_sensors::GpuSensorSource::discover(no_nvidia);
    let mut rapl_src = rapl::RaplSource::discover();
    let mut disk_src = disk_activity::DiskActivitySource::discover();
    let mut net_src = network_stats::NetworkStatsSource::discover();

    // Short sleep for delta-based sources to have meaningful deltas
    thread::sleep(Duration::from_millis(250));

    let mut map = HashMap::new();
    for (id, reading) in hwmon_src.poll() {
        map.insert(id, reading);
    }
    for (id, reading) in freq_src.poll() {
        map.insert(id, reading);
    }
    for (id, reading) in util_src.poll() {
        map.insert(id, reading);
    }
    for (id, reading) in gpu_src.poll() {
        map.insert(id, reading);
    }
    for (id, reading) in rapl_src.poll() {
        map.insert(id, reading);
    }
    for (id, reading) in disk_src.poll() {
        map.insert(id, reading);
    }
    for (id, reading) in net_src.poll() {
        map.insert(id, reading);
    }
    map
}
