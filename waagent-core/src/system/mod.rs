#[derive(Debug)]
pub struct SystemInfo {
    pub hostname: String,
    pub cpu_usage: String,
    pub memory_usage: String,
    pub uptime: String,
    pub os_name: String,
    pub os_version: String,
}

impl SystemInfo {
    pub fn current() -> Self {
        let hostname = get_hostname();
        let cpu_usage = get_cpu_usage_percent();
        let memory_usage = get_memory_usage_percent();
        let uptime = get_uptime();
        let os_version = get_os_version();
        let os_name = get_os_display_name();

        SystemInfo {
            hostname,
            cpu_usage,
            memory_usage,
            uptime,
            os_name,
            os_version,
        }
    }
}

fn get_hostname() -> String {
    sys_info::hostname().unwrap_or_else(|_| "Undefined".to_string())
}

fn get_cpu_usage_percent() -> String {
    // Get CPU load average as a simple approximation
    match sys_info::loadavg() {
        Ok(load) => format!("{:.1}", load.one * 100.0),
        Err(_) => "0".to_string(),
    }
}

fn get_memory_usage_percent() -> String {
    match (sys_info::mem_info(), sys_info::mem_info()) {
        (Ok(mem), _) => {
            let used = mem.total - mem.free;
            let usage_percent = (used as f64 / mem.total as f64) * 100.0;
            format!("{:.1}", usage_percent)
        }
        _ => "0".to_string(),
    }
}

fn get_os_version() -> String {
    let info = os_info::get();
    
    let version = info.version().to_string();
    // There could be two bugs in os_info 3.12.0, if confirmed we need to move this comment
    // to a doc in our repo, submit an issue and if possible submit a patch.
    // version for ubuntu should return "24.04.3" but instead returns "24.4.0"
    version
}

fn get_os_display_name() -> String {
    let info = os_info::get();

    let os_type = info.os_type().to_string().to_lowercase();
    os_type
}

fn get_uptime() -> String {
    #[cfg(all(not(unix), not(windows)))]
    {
        "0".to_string()
    }

    // Simple approximation using boot time
    #[cfg(unix)]    
    {
        match sys_info::boottime() {
            Ok(boot_time) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let uptime = now.saturating_sub(boot_time.tv_sec as u64);
                format!("{}", uptime)
            }
            Err(_) => "0".to_string(),
        }
    }

    #[cfg(windows)]
    {
        use winapi::um::sysinfoapi::GetTickCount64;
        
        unsafe {
            let uptime_ms = GetTickCount64();
            let uptime_seconds = uptime_ms / 1000;
            format!("{}", uptime_seconds)
        }
    }
}