use crate::error::{MsrError, SinfoError};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// Read a Model-Specific Register on a given CPU core.
/// Requires the `msr` kernel module and CAP_SYS_RAWIO or root.
pub fn read_msr(cpu: u32, register: u32) -> crate::error::Result<u64> {
    let path = format!("/dev/cpu/{}/msr", cpu);
    let mut f = File::open(&path).map_err(|e| {
        let msr_err = match e.kind() {
            std::io::ErrorKind::NotFound => MsrError::ModuleNotLoaded,
            std::io::ErrorKind::PermissionDenied => MsrError::PermissionDenied { register },
            _ => MsrError::Io(e),
        };
        SinfoError::Msr {
            cpu,
            source: msr_err,
        }
    })?;
    f.seek(SeekFrom::Start(register as u64))
        .map_err(|e| SinfoError::Msr {
            cpu,
            source: MsrError::Io(e),
        })?;
    let mut buf = [0u8; 8];
    f.read_exact(&mut buf).map_err(|e| SinfoError::Msr {
        cpu,
        source: MsrError::Io(e),
    })?;
    Ok(u64::from_le_bytes(buf))
}

/// Try reading an MSR, returning None on any failure.
pub fn read_msr_optional(cpu: u32, register: u32) -> Option<u64> {
    read_msr(cpu, register).ok()
}
