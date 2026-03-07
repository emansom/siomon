use std::collections::HashMap;
use std::fs;

/// Parse /proc/meminfo into key-value pairs. Values are in bytes.
pub fn parse_meminfo() -> HashMap<String, u64> {
    let Ok(content) = fs::read_to_string("/proc/meminfo") else {
        return HashMap::new();
    };
    parse_meminfo_content(&content)
}

/// Parse meminfo content from a string. Values are in bytes.
pub(crate) fn parse_meminfo_content(content: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let rest = rest.trim();
        let value = if let Some(kb_str) = rest.strip_suffix("kB") {
            kb_str.trim().parse::<u64>().unwrap_or(0) * 1024
        } else {
            rest.parse::<u64>().unwrap_or(0)
        };
        map.insert(key.to_string(), value);
    }
    map
}

/// Parse /proc/cpuinfo into a list of per-processor key-value maps.
pub fn parse_cpuinfo() -> Vec<HashMap<String, String>> {
    let Ok(content) = fs::read_to_string("/proc/cpuinfo") else {
        return Vec::new();
    };
    parse_cpuinfo_content(&content)
}

/// Parse cpuinfo content from a string.
pub(crate) fn parse_cpuinfo_content(content: &str) -> Vec<HashMap<String, String>> {
    let mut processors = Vec::new();
    let mut current: HashMap<String, String> = HashMap::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                processors.push(std::mem::take(&mut current));
            }
            continue;
        }
        if let Some((key, val)) = line.split_once(':') {
            current.insert(key.trim().to_string(), val.trim().to_string());
        }
    }
    if !current.is_empty() {
        processors.push(current);
    }
    processors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_meminfo_basic() {
        let content = "\
MemTotal:       32617396 kB
MemFree:         1234567 kB
MemAvailable:   20000000 kB
Buffers:          456789 kB
SwapTotal:       8388608 kB
SwapFree:        8388608 kB
HugePages_Total:       0
";
        let map = parse_meminfo_content(content);
        assert_eq!(map["MemTotal"], 32617396 * 1024);
        assert_eq!(map["MemFree"], 1234567 * 1024);
        assert_eq!(map["MemAvailable"], 20000000 * 1024);
        assert_eq!(map["SwapTotal"], 8388608 * 1024);
        assert_eq!(map["HugePages_Total"], 0);
    }

    #[test]
    fn test_parse_meminfo_empty() {
        let map = parse_meminfo_content("");
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_cpuinfo_single_processor() {
        let content = "\
processor\t: 0
vendor_id\t: AuthenticAMD
cpu family\t: 25
model\t\t: 33
model name\t: AMD Ryzen 9 5950X
stepping\t: 2
cpu MHz\t\t: 3400.000
";
        let cpus = parse_cpuinfo_content(content);
        assert_eq!(cpus.len(), 1);
        assert_eq!(cpus[0]["processor"], "0");
        assert_eq!(cpus[0]["vendor_id"], "AuthenticAMD");
        assert_eq!(cpus[0]["cpu family"], "25");
        assert_eq!(cpus[0]["model"], "33");
        assert_eq!(cpus[0]["model name"], "AMD Ryzen 9 5950X");
    }

    #[test]
    fn test_parse_cpuinfo_two_processors() {
        let content = "\
processor\t: 0
vendor_id\t: GenuineIntel
model name\t: Intel(R) Core(TM) i7-12700K

processor\t: 1
vendor_id\t: GenuineIntel
model name\t: Intel(R) Core(TM) i7-12700K
";
        let cpus = parse_cpuinfo_content(content);
        assert_eq!(cpus.len(), 2);
        assert_eq!(cpus[0]["processor"], "0");
        assert_eq!(cpus[1]["processor"], "1");
    }

    #[test]
    fn test_parse_cpuinfo_empty() {
        let cpus = parse_cpuinfo_content("");
        assert!(cpus.is_empty());
    }

    #[test]
    fn test_parse_cpuinfo_no_trailing_newline() {
        let content = "processor\t: 0\nvendor_id\t: AuthenticAMD";
        let cpus = parse_cpuinfo_content(content);
        assert_eq!(cpus.len(), 1);
        assert_eq!(cpus[0]["vendor_id"], "AuthenticAMD");
    }
}
