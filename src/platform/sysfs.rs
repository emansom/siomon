use crate::error::{SinfoError, SysfsError};
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_string(path: &Path) -> crate::error::Result<String> {
    fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .map_err(|e| {
            let source = match e.kind() {
                std::io::ErrorKind::NotFound => SysfsError::NotFound,
                std::io::ErrorKind::PermissionDenied => SysfsError::PermissionDenied,
                _ => SysfsError::Io(e),
            };
            SinfoError::Sysfs {
                path: path.to_path_buf(),
                source,
            }
        })
}

pub fn read_u64(path: &Path) -> crate::error::Result<u64> {
    let s = read_string(path)?;
    parse_int_flexible(&s).map_err(|_| SinfoError::Sysfs {
        path: path.to_path_buf(),
        source: SysfsError::Parse {
            expected: "u64",
            actual: s,
        },
    })
}

pub fn read_i64(path: &Path) -> crate::error::Result<i64> {
    let s = read_string(path)?;
    s.parse::<i64>().map_err(|_| SinfoError::Sysfs {
        path: path.to_path_buf(),
        source: SysfsError::Parse {
            expected: "i64",
            actual: s,
        },
    })
}

pub fn read_string_optional(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty()
            || trimmed == "N/A"
            || trimmed == "To Be Filled By O.E.M."
            || trimmed == "Default string"
            || trimmed == "Not Specified"
        {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn read_u64_optional(path: &Path) -> Option<u64> {
    read_string_optional(path).and_then(|s| parse_int_flexible(&s).ok())
}

pub fn read_u32_optional(path: &Path) -> Option<u32> {
    read_u64_optional(path).map(|v| v as u32)
}

pub fn read_link_basename(path: &Path) -> Option<String> {
    fs::read_link(path)
        .ok()
        .and_then(|target| target.file_name().map(|n| n.to_string_lossy().to_string()))
}

pub fn glob_paths(pattern: &str) -> Vec<PathBuf> {
    glob::glob(pattern)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .collect()
}

fn parse_int_flexible(s: &str) -> Result<u64, std::num::ParseIntError> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)
    } else {
        s.parse::<u64>()
    }
}
