use crate::platform::sysfs;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManagementEngine {
    pub firmware_version: Option<String>,
    pub device_path: String,
}

pub fn collect() -> Option<ManagementEngine> {
    // Try Intel MEI (Management Engine Interface)
    let mei_paths = sysfs::glob_paths("/sys/class/mei/mei*");
    for path in mei_paths {
        let fw_ver = sysfs::read_string_optional(&path.join("fw_ver"));
        if fw_ver.is_some() {
            return Some(ManagementEngine {
                firmware_version: fw_ver,
                device_path: path.to_string_lossy().to_string(),
            });
        }
    }

    // Fallback: try reading from /dev/mei0 attributes
    let fw_ver = sysfs::read_string_optional(std::path::Path::new("/sys/class/mei/mei0/fw_ver"));
    if fw_ver.is_some() {
        return Some(ManagementEngine {
            firmware_version: fw_ver,
            device_path: "/sys/class/mei/mei0".to_string(),
        });
    }

    None
}
