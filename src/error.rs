use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SinfoError {
    #[error("sysfs read failed at {path}: {source}")]
    Sysfs { path: PathBuf, source: SysfsError },

    #[error("MSR error on CPU {cpu}: {source}")]
    Msr { cpu: u32, source: MsrError },

    #[error("NVML error: {0}")]
    Nvml(#[from] NvmlError),

    #[error("SMBIOS parse error at offset {offset:#x}: {message}")]
    Smbios { offset: usize, message: String },

    #[error("NVMe ioctl error on {device}: {source}")]
    NvmeIoctl {
        device: PathBuf,
        source: std::io::Error,
    },

    #[error("permission denied: {path} (try running as root)")]
    PermissionDenied { path: PathBuf },

    #[error("hardware not present: {0}")]
    NotPresent(String),

    #[error("CPUID error: {0}")]
    Cpuid(String),

    #[error("output format error: {0}")]
    Output(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
}

#[derive(Debug, Error)]
pub enum SysfsError {
    #[error("file not found")]
    NotFound,

    #[error("permission denied")]
    PermissionDenied,

    #[error("parse error: expected {expected}, got {actual:?}")]
    Parse {
        expected: &'static str,
        actual: String,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum MsrError {
    #[error("MSR module not loaded (/dev/cpu/*/msr not found)")]
    ModuleNotLoaded,

    #[error("permission denied reading MSR {register:#x}")]
    PermissionDenied { register: u32 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum NvmlError {
    #[error("libnvidia-ml.so not found; NVIDIA driver not installed")]
    LibraryNotFound,

    #[error("NVML symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("NVML returned error code {0}")]
    ApiError(u32),

    #[error("NVML initialization failed")]
    InitFailed,
}

pub type Result<T> = std::result::Result<T, SinfoError>;
