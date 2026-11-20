//! BijiOS Activity Monitor — Aesthetic & Comprehensive Real-Time System Monitor
//! 100% Pure Rust + Native GTK4 + Cairo Live Area Charts (Zero External Crates)

use crate::native_de::apps::{create_app_window, read_arch_and_kernel, read_cpu_summary, read_load_avg};
use crate::native_de::gtk4_ffi::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::process::{Command, Stdio};
use std::rc::Rc;

const HISTORY_LEN: usize = 60;

#[derive(Clone, Default)]
struct CpuTimes {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl CpuTimes {
    fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq + self.steal
    }
    fn active(&self) -> u64 {
        self.total().saturating_sub(self.idle + self.iowait)
    }
}

#[derive(Clone, Default)]
struct MemSnapshot {
    total_mb: u64,
    used_mb: u64,
    avail_mb: u64,
    cached_mb: u64,
    buffers_mb: u64,
    swap_total_mb: u64,
    swap_used_mb: u64,
    pct: f64,
}

#[derive(Clone)]
struct NetIfaceStats {
    name: String,
    ipv4: String,
    is_up: bool,
    rx_bytes: u64,
    tx_bytes: u64,
    rx_kbs: f64,
    tx_kbs: f64,
}

#[derive(Clone)]
struct DiskMountInfo {
    device: String,
    fstype: String,
    mount: String,
    used_gb: f64,
    total_gb: f64,
    pct: f64,
}

#[derive(Clone)]
struct ProcEntry {
    pid: u32,
    name: String,
    user: String,
    state: char,
    cpu_pct: f64,
    rss_mb: f64,
    mem_pct: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum SortBy {
    Cpu,
    Mem,
    Pid,
    Name,
}

struct TelemetryState {
    prev_cpu_total: CpuTimes,
    prev_cpu_cores: Vec<CpuTimes>,
    prev_disk_sectors: (u64, u64),
    prev_net_bytes: HashMap<String, (u64, u64)>,
    prev_proc_ticks: HashMap<u32, u64>,
    uid_map: HashMap<u32, String>,

    cpu_pct: f64,
    cpu_user_pct: f64,
    cpu_sys_pct: f64,
    cpu_iowait_pct: f64,
    core_pcts: Vec<f64>,

    mem: MemSnapshot,
    disk_read_mbs: f64,
    disk_write_mbs: f64,
    net_rx_kbs: f64,
    net_tx_kbs: f64,

    cpu_hist: VecDeque<f64>,
    mem_hist: VecDeque<f64>,
    disk_r_hist: VecDeque<f64>,
    disk_w_hist: VecDeque<f64>,
    net_rx_hist: VecDeque<f64>,
    net_tx_hist: VecDeque<f64>,

    ifaces: Vec<NetIfaceStats>,
    mounts: Vec<DiskMountInfo>,
    procs: Vec<ProcEntry>,
    total_procs: usize,
    running_procs: usize,
    sleeping_procs: usize,
}

impl TelemetryState {
    fn new() -> Self {
        let (tot, cores) = read_proc_stat();
        let disk = read_disk_sectors();
        let net = read_net_raw();
        let uid_map = load_uid_map();
        let mut s = Self {
            prev_cpu_total: tot,
            prev_cpu_cores: cores.clone(),
            prev_disk_sectors: disk,
            prev_net_bytes: net,
            prev_proc_ticks: HashMap::new(),
            uid_map,
            cpu_pct: 0.0,
            cpu_user_pct: 0.0,
            cpu_sys_pct: 0.0,
            cpu_iowait_pct: 0.0,
            core_pcts: vec![0.0; cores.len().max(1)],
            mem: read_mem_snapshot(),
            disk_read_mbs: 0.0,
            disk_write_mbs: 0.0,
            net_rx_kbs: 0.0,
            net_tx_kbs: 0.0,
            cpu_hist: VecDeque::from(vec![0.0; HISTORY_LEN]),
            mem_hist: VecDeque::from(vec![0.0; HISTORY_LEN]),
            disk_r_hist: VecDeque::from(vec![0.0; HISTORY_LEN]),
            disk_w_hist: VecDeque::from(vec![0.0; HISTORY_LEN]),
            net_rx_hist: VecDeque::from(vec![0.0; HISTORY_LEN]),
            net_tx_hist: VecDeque::from(vec![0.0; HISTORY_LEN]),
            ifaces: Vec::new(),
            mounts: read_disk_mounts(),
            procs: Vec::new(),
            total_procs: 0,
            running_procs: 0,
            sleeping_procs: 0,
        };
        s.sample(1.0);
        s
    }

    fn sample(&mut self, dt_sec: f64) {
        let dt = dt_sec.max(0.25);

        // 1. CPU delta
        let (cur_tot, cur_cores) = read_proc_stat();
        let d_tot = cur_tot.total().saturating_sub(self.prev_cpu_total.total());
        if d_tot > 0 {
            let d_act = cur_tot.active().saturating_sub(self.prev_cpu_total.active());
            let d_usr = (cur_tot.user + cur_tot.nice)
                .saturating_sub(self.prev_cpu_total.user + self.prev_cpu_total.nice);
            let d_sys = (cur_tot.system + cur_tot.irq + cur_tot.softirq + cur_tot.steal)
                .saturating_sub(
                    self.prev_cpu_total.system
                        + self.prev_cpu_total.irq
                        + self.prev_cpu_total.softirq
                        + self.prev_cpu_total.steal,
                );
            let d_iow = cur_tot.iowait.saturating_sub(self.prev_cpu_total.iowait);
            self.cpu_pct = (d_act as f64 / d_tot as f64 * 100.0).clamp(0.0, 100.0);
            self.cpu_user_pct = (d_usr as f64 / d_tot as f64 * 100.0).clamp(0.0, 100.0);
            self.cpu_sys_pct = (d_sys as f64 / d_tot as f64 * 100.0).clamp(0.0, 100.0);
            self.cpu_iowait_pct = (d_iow as f64 / d_tot as f64 * 100.0).clamp(0.0, 100.0);
        }
        let mut next_core_pcts = Vec::with_capacity(cur_cores.len());
        for (i, c) in cur_cores.iter().enumerate() {
            let prev = self.prev_cpu_cores.get(i).cloned().unwrap_or_default();
            let dt_c = c.total().saturating_sub(prev.total());
            let da_c = c.active().saturating_sub(prev.active());
            let pct = if dt_c > 0 {
                (da_c as f64 / dt_c as f64 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };
            next_core_pcts.push(pct);
        }
        if !next_core_pcts.is_empty() {
            self.core_pcts = next_core_pcts;
        }
        self.prev_cpu_total = cur_tot;
        self.prev_cpu_cores = cur_cores;

        // 2. Memory
        self.mem = read_mem_snapshot();

        // 3. Disk I/O (512-byte sectors -> MB/s)
        let (r_sec, w_sec) = read_disk_sectors();
        let dr = r_sec.saturating_sub(self.prev_disk_sectors.0);
        let dw = w_sec.saturating_sub(self.prev_disk_sectors.1);
        self.disk_read_mbs = (dr as f64 * 512.0) / (1024.0 * 1024.0) / dt;
        self.disk_write_mbs = (dw as f64 * 512.0) / (1024.0 * 1024.0) / dt;
        self.prev_disk_sectors = (r_sec, w_sec);

        // 4. Network RX/TX
        let cur_net = read_net_raw();
        let ipv4_map = read_ipv4_map();
        let mut tot_rx_kbs = 0.0;
        let mut tot_tx_kbs = 0.0;
        let mut ifaces = Vec::new();
        let mut names: Vec<String> = cur_net.keys().cloned().collect();
        names.sort();
        for name in names {
            let (rx, tx) = cur_net[&name];
            let (prx, ptx) = self.prev_net_bytes.get(&name).copied().unwrap_or((rx, tx));
            let rx_kbs = (rx.saturating_sub(prx) as f64 / 1024.0) / dt;
            let tx_kbs = (tx.saturating_sub(ptx) as f64 / 1024.0) / dt;
            if name != "lo" {
                tot_rx_kbs += rx_kbs;
                tot_tx_kbs += tx_kbs;
            }
            let oper = fs::read_to_string(format!("/sys/class/net/{}/operstate", name))
                .unwrap_or_default();
            let is_up = oper.trim() == "up" || (name == "lo" && oper.trim() == "unknown");
            let ipv4 = ipv4_map
                .get(&name)
                .cloned()
                .unwrap_or_else(|| "—".to_string());
            ifaces.push(NetIfaceStats {
                name,
                ipv4,
                is_up,
                rx_bytes: rx,
                tx_bytes: tx,
                rx_kbs,
                tx_kbs,
            });
        }
        self.net_rx_kbs = tot_rx_kbs;
        self.net_tx_kbs = tot_tx_kbs;
        self.ifaces = ifaces;
        self.prev_net_bytes = cur_net;

        // 5. Push rolling histories
        push_hist(&mut self.cpu_hist, self.cpu_pct);
        push_hist(&mut self.mem_hist, self.mem.pct * 100.0);
        push_hist(&mut self.disk_r_hist, self.disk_read_mbs);
        push_hist(&mut self.disk_w_hist, self.disk_write_mbs);
        push_hist(&mut self.net_rx_hist, self.net_rx_kbs);
        push_hist(&mut self.net_tx_hist, self.net_tx_kbs);

        // 6. Processes with per-PID CPU% delta
        let num_cores = self.core_pcts.len().max(1) as f64;
        let mem_total_kb = (self.mem.total_mb * 1024).max(1) as f64;
        let mut next_ticks = HashMap::new();
        let mut procs = Vec::new();
        let mut running = 0usize;
        let mut sleeping = 0usize;

        if let Ok(rd) = fs::read_dir("/proc") {
            for ent in rd.flatten() {
                let Ok(pid) = ent.file_name().to_string_lossy().parse::<u32>() else {
                    continue;
                };
                let pdir = ent.path();
                let Ok(stat_txt) = fs::read_to_string(pdir.join("stat")) else {
                    continue;
                };
                let Some(rparen) = stat_txt.rfind(')') else {
                    continue;
                };
                let Some(lparen) = stat_txt.find('(') else {
                    continue;
                };
                let name = stat_txt[lparen + 1..rparen].to_string();
                let fields: Vec<&str> = stat_txt[rparen + 2..].split_whitespace().collect();
                if fields.len() < 15 {
                    continue;
                }
                let state = fields[0].chars().next().unwrap_or('S');
                if state == 'R' {
                    running += 1;
                } else if state == 'S' || state == 'I' {
                    sleeping += 1;
                }
                let utime: u64 = fields[11].parse().unwrap_or(0);
                let stime: u64 = fields[12].parse().unwrap_or(0);
                let ticks = utime + stime;
                next_ticks.insert(pid, ticks);

                let cpu_pct = if let Some(&prev_t) = self.prev_proc_ticks.get(&pid) {
                    if d_tot > 0 {
                        let dp = ticks.saturating_sub(prev_t);
                        (dp as f64 / d_tot as f64 * 100.0 * num_cores).clamp(0.0, 100.0 * num_cores)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                let mut rss_kb = 0u64;
                let mut uid = 0u32;
                if let Ok(st) = fs::read_to_string(pdir.join("status")) {
                    for l in st.lines() {
                        if l.starts_with("VmRSS:") {
                            rss_kb = l.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
                        } else if l.starts_with("Uid:") {
                            uid = l.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
                        }
                    }
                }
                let user = self
                    .uid_map
                    .get(&uid)
                    .cloned()
                    .unwrap_or_else(|| uid.to_string());
                procs.push(ProcEntry {
                    pid,
                    name,
                    user,
                    state,
                    cpu_pct,
                    rss_mb: rss_kb as f64 / 1024.0,
                    mem_pct: (rss_kb as f64 / mem_total_kb * 100.0).clamp(0.0, 100.0),
                });
            }
        }
        self.prev_proc_ticks = next_ticks;
        self.total_procs = procs.len();
        self.running_procs = running;
        self.sleeping_procs = sleeping;
        self.procs = procs;
    }
}

fn push_hist(q: &mut VecDeque<f64>, val: f64) {
    if q.len() >= HISTORY_LEN {
        q.pop_front();
    }
    q.push_back(val);
}

fn load_uid_map() -> HashMap<u32, String> {
    let mut map = HashMap::new();
    if let Ok(txt) = fs::read_to_string("/etc/passwd") {
        for line in txt.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(uid) = parts[2].parse::<u32>() {
                    map.insert(uid, parts[0].to_string());
                }
            }
        }
    }
    map
}

fn read_proc_stat() -> (CpuTimes, Vec<CpuTimes>) {
    let mut tot = CpuTimes::default();
    let mut cores = Vec::new();
    if let Ok(txt) = fs::read_to_string("/proc/stat") {
        for line in txt.lines() {
            if !line.starts_with("cpu") {
                continue;
            }
            let mut it = line.split_whitespace();
            let Some(label) = it.next() else { continue };
            let nums: Vec<u64> = it.filter_map(|s| s.parse().ok()).collect();
            if nums.len() < 4 {
                continue;
            }
            let ct = CpuTimes {
                user: *nums.first().unwrap_or(&0),
                nice: *nums.get(1).unwrap_or(&0),
                system: *nums.get(2).unwrap_or(&0),
                idle: *nums.get(3).unwrap_or(&0),
                iowait: *nums.get(4).unwrap_or(&0),
                irq: *nums.get(5).unwrap_or(&0),
                softirq: *nums.get(6).unwrap_or(&0),
                steal: *nums.get(7).unwrap_or(&0),
            };
            if label == "cpu" {
                tot = ct;
            } else if label.len() > 3 && label[3..].chars().all(|c| c.is_ascii_digit()) {
                cores.push(ct);
            }
        }
    }
    (tot, cores)
}

fn read_mem_snapshot() -> MemSnapshot {
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    let mut cached_kb = 0u64;
    let mut buffers_kb = 0u64;
    let mut swap_total_kb = 0u64;
    let mut swap_free_kb = 0u64;
    if let Ok(txt) = fs::read_to_string("/proc/meminfo") {
        for line in txt.lines() {
            let val = || line.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
            if line.starts_with("MemTotal:") {
                total_kb = val();
            } else if line.starts_with("MemAvailable:") {
                avail_kb = val();
            } else if line.starts_with("Cached:") {
                cached_kb = val();
            } else if line.starts_with("Buffers:") {
                buffers_kb = val();
            } else if line.starts_with("SwapTotal:") {
                swap_total_kb = val();
            } else if line.starts_with("SwapFree:") {
                swap_free_kb = val();
            }
        }
    }
    let used_kb = total_kb.saturating_sub(avail_kb);
    let swap_used_kb = swap_total_kb.saturating_sub(swap_free_kb);
    let pct = if total_kb > 0 {
        used_kb as f64 / total_kb as f64
    } else {
        0.0
    };
    MemSnapshot {
        total_mb: total_kb / 1024,
        used_mb: used_kb / 1024,
        avail_mb: avail_kb / 1024,
        cached_mb: cached_kb / 1024,
        buffers_mb: buffers_kb / 1024,
        swap_total_mb: swap_total_kb / 1024,
        swap_used_mb: swap_used_kb / 1024,
        pct,
    }
}

fn read_disk_sectors() -> (u64, u64) {
    let mut read_sec = 0u64;
    let mut write_sec = 0u64;
    if let Ok(txt) = fs::read_to_string("/proc/diskstats") {
        for line in txt.lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() < 10 {
                continue;
            }
            let dev = p[2];
            // Only whole disks (vda, sda, nvme0n1) to avoid double counting partitions
            let is_whole = (dev.starts_with("sd") && dev.len() == 3)
                || (dev.starts_with("vd") && dev.len() == 3)
                || (dev.starts_with("nvme") && !dev.contains('p'));
            if is_whole {
                read_sec += p[5].parse::<u64>().unwrap_or(0);
                write_sec += p[9].parse::<u64>().unwrap_or(0);
            }
        }
    }
    (read_sec, write_sec)
}

fn read_net_raw() -> HashMap<String, (u64, u64)> {
    let mut map = HashMap::new();
    if let Ok(txt) = fs::read_to_string("/proc/net/dev") {
        for line in txt.lines().skip(2) {
            if let Some((iface, rest)) = line.split_once(':') {
                let p: Vec<&str> = rest.split_whitespace().collect();
                if p.len() >= 9 {
                    let rx = p[0].parse::<u64>().unwrap_or(0);
                    let tx = p[8].parse::<u64>().unwrap_or(0);
                    map.insert(iface.trim().to_string(), (rx, tx));
                }
            }
        }
    }
    map
}

fn read_ipv4_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(out) = Command::new("ip").args(["-4", "-o", "addr", "show"]).output() {
        let txt = String::from_utf8_lossy(&out.stdout);
        for line in txt.lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 4 {
                map.insert(p[1].to_string(), p[3].to_string());
            }
        }
    }
    map
}

fn read_disk_mounts() -> Vec<DiskMountInfo> {
    let mut out_list = Vec::new();
    if let Ok(out) = Command::new("df")
        .args(["-B1", "-T", "-x", "tmpfs", "-x", "devtmpfs", "-x", "squashfs", "-x", "overlay"])
        .output()
    {
        let txt = String::from_utf8_lossy(&out.stdout);
        for line in txt.lines().skip(1) {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 7 {
                let total_b = p[2].parse::<f64>().unwrap_or(0.0);
                let used_b = p[3].parse::<f64>().unwrap_or(0.0);
                if total_b <= 0.0 {
                    continue;
                }
                let gb = 1024.0 * 1024.0 * 1024.0;
                out_list.push(DiskMountInfo {
                    device: p[0].to_string(),
                    fstype: p[1].to_string(),
                    mount: p[6..].join(" "),
                    used_gb: used_b / gb,
                    total_gb: total_b / gb,
                    pct: (used_b / total_b).clamp(0.0, 1.0),
                });
            }
        }
    }
    out_list
}

fn read_uptime_pretty() -> String {
    if let Ok(txt) = fs::read_to_string("/proc/uptime") {
        if let Some(secs_f) = txt.split_whitespace().next().and_then(|s| s.parse::<f64>().ok()) {
            let s = secs_f as u64;
            let days = s / 86400;
            let hrs = (s % 86400) / 3600;
            let mins = (s % 3600) / 60;
            if days > 0 {
                return format!("Up {}d {}h {}m", days, hrs, mins);
            } else if hrs > 0 {
                return format!("Up {}h {}m", hrs, mins);
            } else {
                return format!("Up {}m", mins);
            }
        }
    }
    "Up 0m".to_string()
}

fn fmt_bytes(b: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let bf = b as f64;
    if bf >= GB {
        format!("{:.2} GB", bf / GB)
    } else if bf >= MB {
        format!("{:.1} MB", bf / MB)
    } else if bf >= KB {
        format!("{:.1} KB", bf / KB)
    } else {
        format!("{} B", b)
    }
}

fn fmt_rate_kbs(kbs: f64) -> String {
    if kbs >= 1024.0 {
        format!("{:.2} MB/s", kbs / 1024.0)
    } else {
        format!("{:.1} KB/s", kbs)
    }
}

// ============================================================================
// Cairo Live Area Chart Renderer (Smooth Translucent Fill + Grid + Glow Dot)
// ============================================================================
unsafe fn draw_series_chart(
    cr: GPointer,
    w: i32,
    h: i32,
    series1: &VecDeque<f64>,
    color1: (f64, f64, f64),
    series2: Option<(&VecDeque<f64>, (f64, f64, f64))>,
    fixed_max: Option<f64>,
) {
    let wf = w as f64;
    let hf = h as f64;
    if wf < 10.0 || hf < 10.0 {
        return;
    }

    // Dark emerald-tinted glass backdrop
    cairo_set_source_rgba(cr, 0.051, 0.133, 0.110, 1.0);
    cairo_rectangle(cr, 0.0, 0.0, wf, hf);
    cairo_fill(cr);

    // Subtle horizontal & vertical grid lines
    cairo_set_source_rgba(cr, 0.498, 0.749, 0.416, 0.12);
    cairo_set_line_width(cr, 1.0);
    for i in 1..4 {
        let y = hf * (i as f64 / 4.0);
        cairo_move_to(cr, 0.0, y);
        cairo_line_to(cr, wf, y);
    }
    for i in 1..6 {
        let x = wf * (i as f64 / 6.0);
        cairo_move_to(cr, x, 0.0);
        cairo_line_to(cr, x, hf);
    }
    cairo_stroke(cr);

    let max_val = if let Some(m) = fixed_max {
        m
    } else {
        let m1 = series1.iter().copied().fold(0.0_f64, f64::max);
        let m2 = series2
            .map(|(s, _)| s.iter().copied().fold(0.0_f64, f64::max))
            .unwrap_or(0.0);
        (m1.max(m2) * 1.2).max(1.0)
    };

    let draw_one = |data: &VecDeque<f64>, (r, g, b): (f64, f64, f64)| {
        let n = data.len();
        if n < 2 {
            return;
        }
        let pad_top = 8.0;
        let pad_bot = 4.0;
        let usable_h = (hf - pad_top - pad_bot).max(4.0);
        let step_x = wf / (n - 1) as f64;

        // Filled translucent area
        cairo_new_path(cr);
        cairo_move_to(cr, 0.0, hf);
        for (i, &val) in data.iter().enumerate() {
            let x = i as f64 * step_x;
            let norm = (val / max_val).clamp(0.0, 1.0);
            let y = pad_top + usable_h * (1.0 - norm);
            cairo_line_to(cr, x, y);
        }
        cairo_line_to(cr, wf, hf);
        cairo_close_path(cr);
        cairo_set_source_rgba(cr, r, g, b, 0.22);
        cairo_fill(cr);

        // Crisp stroke line
        cairo_new_path(cr);
        cairo_set_line_width(cr, 2.2);
        cairo_set_line_join(cr, 1); // CAIRO_LINE_JOIN_ROUND
        cairo_set_line_cap(cr, 1);  // CAIRO_LINE_CAP_ROUND
        let mut last_y = hf;
        for (i, &val) in data.iter().enumerate() {
            let x = i as f64 * step_x;
            let norm = (val / max_val).clamp(0.0, 1.0);
            let y = pad_top + usable_h * (1.0 - norm);
            if i == 0 {
                cairo_move_to(cr, x, y);
            } else {
                cairo_line_to(cr, x, y);
            }
            last_y = y;
        }
        cairo_set_source_rgba(cr, r, g, b, 0.96);
        cairo_stroke(cr);

        // Glowing live endpoint dot
        cairo_new_path(cr);
        cairo_arc(cr, wf - 4.0, last_y, 5.5, 0.0, std::f64::consts::TAU);
        cairo_set_source_rgba(cr, r, g, b, 0.30);
        cairo_fill(cr);

        cairo_new_path(cr);
        cairo_arc(cr, wf - 4.0, last_y, 2.8, 0.0, std::f64::consts::TAU);
        cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.96);
        cairo_fill(cr);
    };

    if let Some((s2, c2)) = series2 {
        draw_one(s2, c2);
    }
    draw_one(series1, color1);
}

unsafe fn set_pressure_badge(lbl: *mut GtkWidget, pct: f64, ok_txt: &str, warn_txt: &str, high_txt: &str) {
    remove_css(lbl, "act-badge-ok");
    remove_css(lbl, "act-badge-warn");
    remove_css(lbl, "act-badge-high");
    if pct >= 80.0 {
        add_css(lbl, "act-badge-high");
        set_label_text(lbl, high_txt);
    } else if pct >= 55.0 {
        add_css(lbl, "act-badge-warn");
        set_label_text(lbl, warn_txt);
    } else {
        add_css(lbl, "act-badge-ok");
        set_label_text(lbl, ok_txt);
    }
}

unsafe fn make_chart_widget(height: i32) -> (*mut GtkWidget, *mut GtkWidget) {
    let frame = new_box(GTK_ORIENTATION_VERTICAL, 0, &["act-chart-frame"]);
    let area = gtk_drawing_area_new();
    gtk_drawing_area_set_content_height(area, height);
    gtk_widget_set_hexpand(area, 1);
    gtk_box_append(frame, area);
    (frame, area)
}

pub fn open_activity_monitor(app: *mut GtkApplication) {
    unsafe {
        let win = create_app_window(app, "📈 BijiOS Activity Monitor", 980, 680);
        let root = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);

        let state = Rc::new(RefCell::new(TelemetryState::new()));
        let paused = Rc::new(Cell::new(false));
        let sort_by = Rc::new(Cell::new(SortBy::Cpu));
        let filter_query = Rc::new(RefCell::new(String::new()));
        let active_tab = Rc::new(RefCell::new("overview".to_string()));

        // ====================================================================
        // TOP CONTROL BAR (Segmented Tabs + Live Uptime + Pause/Resume)
        // ====================================================================
        let topbar = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &["act-topbar"]);
        let tabs_box = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
        gtk_widget_set_hexpand(tabs_box, 1);

        let stack = gtk_stack_new();
        gtk_widget_set_hexpand(stack, 1);
        gtk_widget_set_vexpand(stack, 1);

        let tab_defs: [(&str, &str); 6] = [
            ("overview", "⚡ Overview"),
            ("cpu", "🔥 CPU"),
            ("memory", "🧠 Memory"),
            ("disk", "💾 Disk"),
            ("network", "🌐 Network"),
            ("processes", "📋 Processes"),
        ];
        let tab_btns: Rc<RefCell<Vec<(String, *mut GtkWidget)>>> = Rc::new(RefCell::new(Vec::new()));
        for (idx, (id, label)) in tab_defs.iter().enumerate() {
            let css: &[&str] = if idx == 0 { &["seg-btn-active"] } else { &["seg-btn"] };
            let btn = new_button(label, css);
            gtk_box_append(tabs_box, btn);
            tab_btns.borrow_mut().push((id.to_string(), btn));

            let all_btns = Rc::clone(&tab_btns);
            let at = Rc::clone(&active_tab);
            let tid = id.to_string();
            connect_signal(btn, "clicked", move || {
                *at.borrow_mut() = tid.clone();
                let cname = cstr(&tid);
                gtk_stack_set_visible_child_name(stack, cname.as_ptr());
                for (k, b) in all_btns.borrow().iter() {
                    remove_css(*b, "seg-btn");
                    remove_css(*b, "seg-btn-active");
                    if k == &tid {
                        add_css(*b, "seg-btn-active");
                    } else {
                        add_css(*b, "seg-btn");
                    }
                }
            });
        }

        let uptime_pill = new_label(&read_uptime_pretty(), &["act-uptime-pill"]);
        let pause_btn = new_button("⏸ Pause", &["seg-btn"]);
        {
            let p = Rc::clone(&paused);
            connect_signal(pause_btn, "clicked", move || {
                let next = !p.get();
                p.set(next);
                if next {
                    let lbl = cstr("▶ Resume");
                    gtk_button_set_label(pause_btn, lbl.as_ptr());
                    remove_css(pause_btn, "seg-btn");
                    add_css(pause_btn, "seg-btn-active");
                } else {
                    let lbl = cstr("⏸ Pause");
                    gtk_button_set_label(pause_btn, lbl.as_ptr());
                    remove_css(pause_btn, "seg-btn-active");
                    add_css(pause_btn, "seg-btn");
                }
            });
        }

        gtk_box_append(topbar, tabs_box);
        gtk_box_append(topbar, uptime_pill);
        gtk_box_append(topbar, pause_btn);
        gtk_box_append(root, topbar);

        // ====================================================================
        // TAB 1: OVERVIEW (4 KPI Cards + 2x2 Live Cairo Charts + Sys Info)
        // ====================================================================
        let ov_scroll = gtk_scrolled_window_new();
        gtk_scrolled_window_set_policy(ov_scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
        let ov_page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
        set_margin_all(ov_page, 18);

        // 4 KPI Summary Cards
        let kpi_row = new_box(GTK_ORIENTATION_HORIZONTAL, 12, &[]);
        gtk_box_set_homogeneous(kpi_row, 1);

        let mk_kpi = |title: &str, init_val: &str, init_sub: &str| {
            let card = new_box(GTK_ORIENTATION_VERTICAL, 5, &["act-kpi-card"]);
            let hdr = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
            let t = new_label(title, &["app-section-title"]);
            gtk_label_set_xalign(t, 0.0);
            gtk_widget_set_hexpand(t, 1);
            let badge = new_label("Normal", &["act-badge-ok"]);
            gtk_box_append(hdr, t);
            gtk_box_append(hdr, badge);
            let val = new_label(init_val, &["act-kpi-val"]);
            gtk_label_set_xalign(val, 0.0);
            let sub = new_label(init_sub, &["act-kpi-sub"]);
            gtk_label_set_xalign(sub, 0.0);
            gtk_box_append(card, hdr);
            gtk_box_append(card, val);
            gtk_box_append(card, sub);
            (card, val, sub, badge)
        };

        let (k_cpu_card, k_cpu_val, k_cpu_sub, k_cpu_badge) = mk_kpi("⚡ CPU", "0.0%", "Load: 0.00");
        let (k_mem_card, k_mem_val, k_mem_sub, k_mem_badge) = mk_kpi("🧠 MEMORY", "0%", "0 / 0 MB");
        let (k_dsk_card, k_dsk_val, k_dsk_sub, k_dsk_badge) = mk_kpi("💾 DISK I/O", "0.0 MB/s", "R: 0.0 • W: 0.0");
        let (k_net_card, k_net_val, k_net_sub, k_net_badge) = mk_kpi("🌐 NETWORK", "0.0 KB/s", "↓ 0.0 • ↑ 0.0");

        gtk_box_append(kpi_row, k_cpu_card);
        gtk_box_append(kpi_row, k_mem_card);
        gtk_box_append(kpi_row, k_dsk_card);
        gtk_box_append(kpi_row, k_net_card);
        gtk_box_append(ov_page, kpi_row);

        // 2x2 Live Cairo Charts Grid
        let charts_grid = gtk_grid_new();
        gtk_grid_set_row_spacing(charts_grid, 12);
        gtk_grid_set_column_spacing(charts_grid, 12);
        gtk_grid_set_column_homogeneous(charts_grid, 1);

        let mk_chart_card = |title: &str, legend_text: &str, legend_css: &str, h: i32| {
            let card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
            let hdr = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
            let t = new_label(title, &["app-section-title"]);
            gtk_label_set_xalign(t, 0.0);
            gtk_widget_set_hexpand(t, 1);
            let leg = new_label(legend_text, &[legend_css]);
            gtk_box_append(hdr, t);
            gtk_box_append(hdr, leg);
            let (frame, area) = make_chart_widget(h);
            gtk_box_append(card, hdr);
            gtk_box_append(card, frame);
            (card, leg, area)
        };

        let (ov_cpu_c, ov_cpu_leg, ov_cpu_area) = mk_chart_card("CPU UTILIZATION (60s)", "0.0%", "act-legend-cpu", 125);
        let (ov_mem_c, ov_mem_leg, ov_mem_area) = mk_chart_card("MEMORY PRESSURE (60s)", "0.0%", "act-legend-mem", 125);
        let (ov_dsk_c, ov_dsk_leg, ov_dsk_area) = mk_chart_card("DISK THROUGHPUT (60s)", "R 0.0 / W 0.0 MB/s", "act-legend-read", 125);
        let (ov_net_c, ov_net_leg, ov_net_area) = mk_chart_card("NETWORK TRAFFIC (60s)", "↓ 0.0 / ↑ 0.0 KB/s", "act-legend-rx", 125);

        gtk_grid_attach(charts_grid, ov_cpu_c, 0, 0, 1, 1);
        gtk_grid_attach(charts_grid, ov_mem_c, 1, 0, 1, 1);
        gtk_grid_attach(charts_grid, ov_dsk_c, 0, 1, 1, 1);
        gtk_grid_attach(charts_grid, ov_net_c, 1, 1, 1, 1);
        gtk_box_append(ov_page, charts_grid);

        // System Telemetry Strip
        let sys_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
        let sys_hdr = new_label("🖥️ SYSTEM TELEMETRY SUMMARY", &["app-section-title"]);
        gtk_label_set_xalign(sys_hdr, 0.0);
        gtk_box_append(sys_card, sys_hdr);

        let hostname = fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "BijiOS".into());
        let (arch, kernel) = read_arch_and_kernel();
        let cpu_model = read_cpu_summary();

        let sys_grid = gtk_grid_new();
        gtk_grid_set_row_spacing(sys_grid, 6);
        gtk_grid_set_column_spacing(sys_grid, 24);
        gtk_grid_set_column_homogeneous(sys_grid, 1);

        let mk_kv = |k: &str, v: &str| {
            let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
            let kl = new_label(k, &["kv-key"]);
            gtk_label_set_xalign(kl, 0.0);
            gtk_widget_set_size_request(kl, 110, -1);
            let vl = new_label(v, &["kv-val"]);
            gtk_label_set_xalign(vl, 0.0);
            gtk_label_set_ellipsize(vl, 3);
            gtk_widget_set_hexpand(vl, 1);
            gtk_box_append(row, kl);
            gtk_box_append(row, vl);
            (row, vl)
        };

        let (r_host, _) = mk_kv("Host / OS", &format!("{} • BijiOS 1.0.0 \"Padi\"", hostname.trim()));
        let (r_kern, _) = mk_kv("Kernel", &format!("{} ({})", kernel, arch));
        let (r_cpu, _) = mk_kv("Processor", &cpu_model);
        let (r_load, sys_load_lbl) = mk_kv("Load Avg", &read_load_avg());
        let (r_proc, sys_proc_lbl) = mk_kv("Processes", "0 total");
        let (r_swap, sys_swap_lbl) = mk_kv("Swap Usage", "0 MB / 0 MB");

        gtk_grid_attach(sys_grid, r_host, 0, 0, 1, 1);
        gtk_grid_attach(sys_grid, r_kern, 1, 0, 1, 1);
        gtk_grid_attach(sys_grid, r_cpu, 0, 1, 1, 1);
        gtk_grid_attach(sys_grid, r_load, 1, 1, 1, 1);
        gtk_grid_attach(sys_grid, r_proc, 0, 2, 1, 1);
        gtk_grid_attach(sys_grid, r_swap, 1, 2, 1, 1);
        gtk_box_append(sys_card, sys_grid);
        gtk_box_append(ov_page, sys_card);

        gtk_scrolled_window_set_child(ov_scroll, ov_page);
        let ov_name = cstr("overview");
        gtk_stack_add_named(stack, ov_scroll, ov_name.as_ptr());

        // ====================================================================
        // TAB 2: CPU (Large Chart + Breakdown + Per-Core Bars + Top CPU)
        // ====================================================================
        let cpu_scroll = gtk_scrolled_window_new();
        gtk_scrolled_window_set_policy(cpu_scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
        let cpu_page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
        set_margin_all(cpu_page, 18);

        let (cpu_big_c, cpu_big_leg, cpu_big_area) = mk_chart_card("CPU UTILIZATION HISTORY", "Total: 0.0%", "act-legend-cpu", 175);
        let cpu_break_lbl = new_label("User: 0.0%  •  System: 0.0%  •  I/O Wait: 0.0%  •  Idle: 100.0%", &["app-muted"]);
        gtk_label_set_xalign(cpu_break_lbl, 0.0);
        gtk_box_append(cpu_big_c, cpu_break_lbl);
        gtk_box_append(cpu_page, cpu_big_c);

        let cores_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
        let cores_hdr = new_label("⚡ PER-CORE UTILIZATION", &["app-section-title"]);
        gtk_label_set_xalign(cores_hdr, 0.0);
        gtk_box_append(cores_card, cores_hdr);

        let cores_grid = gtk_grid_new();
        gtk_grid_set_row_spacing(cores_grid, 8);
        gtk_grid_set_column_spacing(cores_grid, 14);
        gtk_grid_set_column_homogeneous(cores_grid, 1);
        let core_count = state.borrow().core_pcts.len().max(1);
        let mut core_widgets: Vec<(*mut GtkWidget, *mut GtkWidget)> = Vec::new();
        for i in 0..core_count {
            let cell = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
            let top = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
            let name_l = new_label(&format!("Core {}", i), &["kv-key"]);
            gtk_label_set_xalign(name_l, 0.0);
            gtk_widget_set_hexpand(name_l, 1);
            let pct_l = new_label("0.0%", &["act-proc-mono"]);
            gtk_box_append(top, name_l);
            gtk_box_append(top, pct_l);
            let bar = gtk_progress_bar_new();
            add_css(bar, "act-bar-cpu");
            gtk_box_append(cell, top);
            gtk_box_append(cell, bar);
            gtk_grid_attach(cores_grid, cell, (i % 2) as i32, (i / 2) as i32, 1, 1);
            core_widgets.push((pct_l, bar));
        }
        gtk_box_append(cores_card, cores_grid);
        gtk_box_append(cpu_page, cores_card);

        let top_cpu_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
        let tc_hdr = new_label("🔥 TOP CPU PROCESSES", &["app-section-title"]);
        gtk_label_set_xalign(tc_hdr, 0.0);
        let top_cpu_list = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
        gtk_box_append(top_cpu_card, tc_hdr);
        gtk_box_append(top_cpu_card, top_cpu_list);
        gtk_box_append(cpu_page, top_cpu_card);

        gtk_scrolled_window_set_child(cpu_scroll, cpu_page);
        let cpu_name = cstr("cpu");
        gtk_stack_add_named(stack, cpu_scroll, cpu_name.as_ptr());

        // ====================================================================
        // TAB 3: MEMORY (Large Chart + Detailed Breakdown + Top Memory)
        // ====================================================================
        let mem_scroll = gtk_scrolled_window_new();
        gtk_scrolled_window_set_policy(mem_scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
        let mem_page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
        set_margin_all(mem_page, 18);

        let (mem_big_c, mem_big_leg, mem_big_area) = mk_chart_card("MEMORY USAGE HISTORY", "0.0%", "act-legend-mem", 175);
        let mem_big_bar = gtk_progress_bar_new();
        add_css(mem_big_bar, "act-bar-mem");
        gtk_box_append(mem_big_c, mem_big_bar);
        gtk_box_append(mem_page, mem_big_c);

        let mem_det_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
        let md_hdr = new_label("🧠 MEMORY & SWAP BREAKDOWN", &["app-section-title"]);
        gtk_label_set_xalign(md_hdr, 0.0);
        gtk_box_append(mem_det_card, md_hdr);

        let mem_grid = gtk_grid_new();
        gtk_grid_set_row_spacing(mem_grid, 6);
        gtk_grid_set_column_spacing(mem_grid, 24);
        gtk_grid_set_column_homogeneous(mem_grid, 1);

        let (rm_used, m_used_lbl) = mk_kv("Used RAM", "0 MB");
        let (rm_avail, m_avail_lbl) = mk_kv("Available RAM", "0 MB");
        let (rm_cache, m_cache_lbl) = mk_kv("Page Cache", "0 MB");
        let (rm_buf, m_buf_lbl) = mk_kv("Buffers", "0 MB");
        let (rm_tot, m_tot_lbl) = mk_kv("Total Installed", "0 MB");
        let (rm_swp, m_swp_lbl) = mk_kv("Swap Used", "0 MB / 0 MB");

        gtk_grid_attach(mem_grid, rm_used, 0, 0, 1, 1);
        gtk_grid_attach(mem_grid, rm_avail, 1, 0, 1, 1);
        gtk_grid_attach(mem_grid, rm_cache, 0, 1, 1, 1);
        gtk_grid_attach(mem_grid, rm_buf, 1, 1, 1, 1);
        gtk_grid_attach(mem_grid, rm_tot, 0, 2, 1, 1);
        gtk_grid_attach(mem_grid, rm_swp, 1, 2, 1, 1);
        gtk_box_append(mem_det_card, mem_grid);
        gtk_box_append(mem_page, mem_det_card);

        let top_mem_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
        let tm_hdr = new_label("🧠 TOP MEMORY PROCESSES", &["app-section-title"]);
        gtk_label_set_xalign(tm_hdr, 0.0);
        let top_mem_list = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
        gtk_box_append(top_mem_card, tm_hdr);
        gtk_box_append(top_mem_card, top_mem_list);
        gtk_box_append(mem_page, top_mem_card);

        gtk_scrolled_window_set_child(mem_scroll, mem_page);
        let mem_name = cstr("memory");
        gtk_stack_add_named(stack, mem_scroll, mem_name.as_ptr());

        // ====================================================================
        // TAB 4: DISK (Dual Read/Write Chart + Mounted Filesystems)
        // ====================================================================
        let dsk_scroll = gtk_scrolled_window_new();
        gtk_scrolled_window_set_policy(dsk_scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
        let dsk_page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
        set_margin_all(dsk_page, 18);

        let (dsk_big_c, dsk_big_leg, dsk_big_area) = mk_chart_card(
            "DISK READ / WRITE THROUGHPUT (60s)",
            "Amber: Read • Rose: Write",
            "act-legend-read",
            175,
        );
        gtk_box_append(dsk_page, dsk_big_c);

        let mounts_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
        let mnt_hdr = new_label("💾 MOUNTED FILESYSTEMS", &["app-section-title"]);
        gtk_label_set_xalign(mnt_hdr, 0.0);
        let mounts_list = new_box(GTK_ORIENTATION_VERTICAL, 8, &[]);
        gtk_box_append(mounts_card, mnt_hdr);
        gtk_box_append(mounts_card, mounts_list);
        gtk_box_append(dsk_page, mounts_card);

        gtk_scrolled_window_set_child(dsk_scroll, dsk_page);
        let dsk_name = cstr("disk");
        gtk_stack_add_named(stack, dsk_scroll, dsk_name.as_ptr());

        // ====================================================================
        // TAB 5: NETWORK (Dual RX/TX Chart + Active Interfaces)
        // ====================================================================
        let net_scroll = gtk_scrolled_window_new();
        gtk_scrolled_window_set_policy(net_scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
        let net_page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
        set_margin_all(net_page, 18);

        let (net_big_c, net_big_leg, net_big_area) = mk_chart_card(
            "NETWORK DOWNLOAD / UPLOAD (60s)",
            "Emerald: RX • Indigo: TX",
            "act-legend-rx",
            175,
        );
        gtk_box_append(net_page, net_big_c);

        let ifaces_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
        let if_hdr = new_label("🌐 NETWORK INTERFACES", &["app-section-title"]);
        gtk_label_set_xalign(if_hdr, 0.0);
        let ifaces_list = new_box(GTK_ORIENTATION_VERTICAL, 6, &[]);
        gtk_box_append(ifaces_card, if_hdr);
        gtk_box_append(ifaces_card, ifaces_list);
        gtk_box_append(net_page, ifaces_card);

        gtk_scrolled_window_set_child(net_scroll, net_page);
        let net_name = cstr("network");
        gtk_stack_add_named(stack, net_scroll, net_name.as_ptr());

        // ====================================================================
        // TAB 6: INTERACTIVE PROCESS MANAGER (Search + Sort + End / Kill)
        // ====================================================================
        let proc_page = new_box(GTK_ORIENTATION_VERTICAL, 10, &[]);
        set_margin_all(proc_page, 16);

        let proc_ctrl = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
        let search_entry = gtk_entry_new();
        let sph = cstr("🔍 Filter processes by name, user, or PID...");
        gtk_entry_set_placeholder_text(search_entry, sph.as_ptr());
        add_css(search_entry, "app-entry");
        gtk_widget_set_hexpand(search_entry, 1);
        gtk_box_append(proc_ctrl, search_entry);

        let sort_box = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &[]);
        let sort_defs = [
            (SortBy::Cpu, "CPU %"),
            (SortBy::Mem, "Memory"),
            (SortBy::Pid, "PID"),
            (SortBy::Name, "Name"),
        ];
        let sort_btns: Rc<RefCell<Vec<(SortBy, *mut GtkWidget)>>> = Rc::new(RefCell::new(Vec::new()));
        for (i, (sb, lbl)) in sort_defs.iter().enumerate() {
            let css: &[&str] = if i == 0 { &["seg-btn-active"] } else { &["seg-btn"] };
            let b = new_button(lbl, css);
            gtk_box_append(sort_box, b);
            sort_btns.borrow_mut().push((*sb, b));
        }
        gtk_box_append(proc_ctrl, sort_box);
        gtk_box_append(proc_page, proc_ctrl);

        let proc_meta_row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
        let proc_count_lbl = new_label("Processes: 0", &["app-muted"]);
        gtk_label_set_xalign(proc_count_lbl, 0.0);
        gtk_widget_set_hexpand(proc_count_lbl, 1);
        let proc_status_lbl = new_label("", &["act-badge-ok"]);
        gtk_widget_set_visible(proc_status_lbl, 0);
        gtk_box_append(proc_meta_row, proc_count_lbl);
        gtk_box_append(proc_meta_row, proc_status_lbl);
        gtk_box_append(proc_page, proc_meta_row);

        // Table Header
        let p_hdr = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &["act-proc-header"]);
        let add_hdr_col = |txt: &str, w: i32, expand: bool| {
            let l = new_label(txt, &["act-proc-hdr-lbl"]);
            gtk_label_set_xalign(l, 0.0);
            if expand {
                gtk_widget_set_hexpand(l, 1);
            } else {
                gtk_widget_set_size_request(l, w, -1);
            }
            gtk_box_append(p_hdr, l);
        };
        add_hdr_col("PROCESS NAME", 0, true);
        add_hdr_col("PID", 66, false);
        add_hdr_col("USER", 88, false);
        add_hdr_col("STATE", 74, false);
        add_hdr_col("CPU %", 68, false);
        add_hdr_col("MEMORY", 96, false);
        add_hdr_col("ACTIONS", 116, false);
        gtk_box_append(proc_page, p_hdr);

        let proc_scroll = gtk_scrolled_window_new();
        gtk_scrolled_window_set_policy(proc_scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
        gtk_widget_set_hexpand(proc_scroll, 1);
        gtk_widget_set_vexpand(proc_scroll, 1);
        let proc_rows_box = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
        gtk_scrolled_window_set_child(proc_scroll, proc_rows_box);
        gtk_box_append(proc_page, proc_scroll);

        let proc_name = cstr("processes");
        gtk_stack_add_named(stack, proc_page, proc_name.as_ptr());

        gtk_box_append(root, stack);

        // ====================================================================
        // WIRE CAIRO DRAW FUNCTIONS TO ALL 8 CHARTS
        // ====================================================================
        let bind_cpu_chart = |area: *mut GtkWidget| {
            let st = Rc::clone(&state);
            set_drawing_area_draw_func(area, move |cr, w, h| {
                let s = st.borrow();
                draw_series_chart(cr, w, h, &s.cpu_hist, (0.498, 0.749, 0.416), None, Some(100.0));
            });
        };
        let bind_mem_chart = |area: *mut GtkWidget| {
            let st = Rc::clone(&state);
            set_drawing_area_draw_func(area, move |cr, w, h| {
                let s = st.borrow();
                draw_series_chart(cr, w, h, &s.mem_hist, (0.231, 0.510, 0.965), None, Some(100.0));
            });
        };
        let bind_dsk_chart = |area: *mut GtkWidget| {
            let st = Rc::clone(&state);
            set_drawing_area_draw_func(area, move |cr, w, h| {
                let s = st.borrow();
                draw_series_chart(
                    cr,
                    w,
                    h,
                    &s.disk_r_hist,
                    (0.910, 0.639, 0.239),
                    Some((&s.disk_w_hist, (0.882, 0.114, 0.282))),
                    None,
                );
            });
        };
        let bind_net_chart = |area: *mut GtkWidget| {
            let st = Rc::clone(&state);
            set_drawing_area_draw_func(area, move |cr, w, h| {
                let s = st.borrow();
                draw_series_chart(
                    cr,
                    w,
                    h,
                    &s.net_rx_hist,
                    (0.063, 0.725, 0.506),
                    Some((&s.net_tx_hist, (0.388, 0.400, 0.945))),
                    None,
                );
            });
        };

        bind_cpu_chart(ov_cpu_area);
        bind_mem_chart(ov_mem_area);
        bind_dsk_chart(ov_dsk_area);
        bind_net_chart(ov_net_area);
        bind_cpu_chart(cpu_big_area);
        bind_mem_chart(mem_big_area);
        bind_dsk_chart(dsk_big_area);
        bind_net_chart(net_big_area);

        // ====================================================================
        // RENDER PROCESSES HELPER (reused by search, sort, and timer)
        // ====================================================================
        let my_pid = std::process::id();
        let render_procs: Rc<dyn Fn()> = {
            let st = Rc::clone(&state);
            let sb = Rc::clone(&sort_by);
            let fq = Rc::clone(&filter_query);
            Rc::new(move || {
                let s = st.borrow();
                let q = fq.borrow().to_lowercase();
                let mut list: Vec<ProcEntry> = s
                    .procs
                    .iter()
                    .filter(|p| {
                        q.is_empty()
                            || p.name.to_lowercase().contains(&q)
                            || p.user.to_lowercase().contains(&q)
                            || p.pid.to_string().contains(&q)
                    })
                    .cloned()
                    .collect();

                match sb.get() {
                    SortBy::Cpu => list.sort_by(|a, b| {
                        b.cpu_pct
                            .partial_cmp(&a.cpu_pct)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| b.rss_mb.partial_cmp(&a.rss_mb).unwrap_or(std::cmp::Ordering::Equal))
                    }),
                    SortBy::Mem => list.sort_by(|a, b| {
                        b.rss_mb.partial_cmp(&a.rss_mb).unwrap_or(std::cmp::Ordering::Equal)
                    }),
                    SortBy::Pid => list.sort_by_key(|p| p.pid),
                    SortBy::Name => list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                }

                set_label_text(
                    proc_count_lbl,
                    &format!(
                        "Showing {} of {} processes  •  {} running  •  {} sleeping",
                        list.len().min(45),
                        s.total_procs,
                        s.running_procs,
                        s.sleeping_procs
                    ),
                );

                clear_box_children(proc_rows_box);
                for (idx, p) in list.into_iter().take(45).enumerate() {
                    let row_css = if idx % 2 == 0 { "act-proc-row" } else { "act-proc-row-alt" };
                    let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[row_css]);

                    let name_l = new_label(&p.name, &["act-proc-name"]);
                    gtk_label_set_xalign(name_l, 0.0);
                    gtk_label_set_ellipsize(name_l, 3);
                    gtk_widget_set_hexpand(name_l, 1);

                    let pid_l = new_label(&p.pid.to_string(), &["act-proc-mono"]);
                    gtk_label_set_xalign(pid_l, 0.0);
                    gtk_widget_set_size_request(pid_l, 66, -1);

                    let usr_l = new_label(&p.user, &["kv-val"]);
                    gtk_label_set_xalign(usr_l, 0.0);
                    gtk_label_set_ellipsize(usr_l, 3);
                    gtk_widget_set_size_request(usr_l, 88, -1);

                    let (st_txt, st_css) = match p.state {
                        'R' => ("Running", "act-state-run"),
                        'S' | 'I' => ("Sleep", "act-state-sleep"),
                        'Z' => ("Zombie", "act-badge-high"),
                        'D' => ("Disk", "act-state-other"),
                        _ => ("Other", "act-state-other"),
                    };
                    let st_box = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
                    gtk_widget_set_size_request(st_box, 74, -1);
                    let st_l = new_label(st_txt, &[st_css]);
                    gtk_widget_set_valign(st_l, GTK_ALIGN_CENTER);
                    gtk_box_append(st_box, st_l);

                    let cpu_l = new_label(&format!("{:.1}%", p.cpu_pct), &["act-proc-mono"]);
                    gtk_label_set_xalign(cpu_l, 0.0);
                    gtk_widget_set_size_request(cpu_l, 68, -1);

                    let mem_l = new_label(&format!("{:.1} MB", p.rss_mb), &["act-proc-mono"]);
                    gtk_label_set_xalign(mem_l, 0.0);
                    gtk_widget_set_size_request(mem_l, 96, -1);

                    let act_box = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &[]);
                    gtk_widget_set_size_request(act_box, 116, -1);
                    let protected = p.pid <= 2 || p.pid == my_pid;
                    let end_btn = new_button("End", &["act-end-btn"]);
                    let kill_btn = new_button("Kill", &["act-kill-btn"]);
                    if protected {
                        gtk_widget_set_sensitive(end_btn, 0);
                        gtk_widget_set_sensitive(kill_btn, 0);
                    } else {
                        let pid_end = p.pid;
                        let pname_end = p.name.clone();
                        connect_signal(end_btn, "clicked", move || {
                            let _ = Command::new("kill")
                                .args(["-TERM", &pid_end.to_string()])
                                .stdin(Stdio::null())
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .status();
                            set_label_text(proc_status_lbl, &format!("Sent SIGTERM to {} (PID {})", pname_end, pid_end));
                            gtk_widget_set_visible(proc_status_lbl, 1);
                        });

                        let pid_kill = p.pid;
                        let pname_kill = p.name.clone();
                        connect_signal(kill_btn, "clicked", move || {
                            let _ = Command::new("kill")
                                .args(["-KILL", &pid_kill.to_string()])
                                .stdin(Stdio::null())
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .status();
                            set_label_text(proc_status_lbl, &format!("Force killed {} (PID {})", pname_kill, pid_kill));
                            gtk_widget_set_visible(proc_status_lbl, 1);
                        });
                    }
                    gtk_box_append(act_box, end_btn);
                    gtk_box_append(act_box, kill_btn);

                    gtk_box_append(row, name_l);
                    gtk_box_append(row, pid_l);
                    gtk_box_append(row, usr_l);
                    gtk_box_append(row, st_box);
                    gtk_box_append(row, cpu_l);
                    gtk_box_append(row, mem_l);
                    gtk_box_append(row, act_box);
                    gtk_box_append(proc_rows_box, row);
                }
            })
        };

        // Wire Sort Buttons & Search Entry
        for (sb_val, btn) in sort_btns.borrow().iter().cloned() {
            let all = Rc::clone(&sort_btns);
            let sb = Rc::clone(&sort_by);
            let rp = Rc::clone(&render_procs);
            connect_signal(btn, "clicked", move || {
                sb.set(sb_val);
                for (k, b) in all.borrow().iter() {
                    remove_css(*b, "seg-btn");
                    remove_css(*b, "seg-btn-active");
                    if *k == sb_val {
                        add_css(*b, "seg-btn-active");
                    } else {
                        add_css(*b, "seg-btn");
                    }
                }
                rp();
            });
        }
        {
            let fq = Rc::clone(&filter_query);
            let rp = Rc::clone(&render_procs);
            connect_signal(search_entry, "changed", move || {
                *fq.borrow_mut() = get_entry_text(search_entry);
                rp();
            });
        }

        // ====================================================================
        // MAIN 1-SECOND TELEMETRY REFRESH LOOP (Destroy-Safe)
        // ====================================================================
        let render_all = {
            let st = Rc::clone(&state);
            let rp = Rc::clone(&render_procs);
            let at = Rc::clone(&active_tab);
            let tick_count = Rc::new(Cell::new(0u32));
            move || {
                let tc = tick_count.get().wrapping_add(1);
                tick_count.set(tc);
                {
                    let mut s = st.borrow_mut();
                    s.sample(1.0);
                    if tc % 10 == 0 {
                        s.mounts = read_disk_mounts();
                    }
                }
                let s = st.borrow();

                // Top uptime pill
                set_label_text(uptime_pill, &read_uptime_pretty());

                // Overview KPI cards
                let load_str = read_load_avg();
                set_label_text(k_cpu_val, &format!("{:.1}%", s.cpu_pct));
                set_label_text(k_cpu_sub, &format!("Load: {}", load_str));
                set_pressure_badge(k_cpu_badge, s.cpu_pct, "Healthy", "Busy", "High");

                let mem_pct100 = s.mem.pct * 100.0;
                set_label_text(k_mem_val, &format!("{:.0}%", mem_pct100));
                set_label_text(k_mem_sub, &format!("{} / {} MB", s.mem.used_mb, s.mem.total_mb));
                set_pressure_badge(k_mem_badge, mem_pct100, "Low", "Moderate", "High");

                let dsk_tot = s.disk_read_mbs + s.disk_write_mbs;
                set_label_text(k_dsk_val, &format!("{:.1} MB/s", dsk_tot));
                set_label_text(k_dsk_sub, &format!("R: {:.1} • W: {:.1} MB/s", s.disk_read_mbs, s.disk_write_mbs));
                set_pressure_badge(k_dsk_badge, (dsk_tot * 2.0).min(100.0), "Idle", "Active", "Heavy");

                let net_tot = s.net_rx_kbs + s.net_tx_kbs;
                set_label_text(k_net_val, &fmt_rate_kbs(net_tot));
                set_label_text(k_net_sub, &format!("↓ {} • ↑ {}", fmt_rate_kbs(s.net_rx_kbs), fmt_rate_kbs(s.net_tx_kbs)));
                set_pressure_badge(k_net_badge, (net_tot / 100.0).min(100.0), "Quiet", "Active", "Heavy");

                // Chart legends
                set_label_text(ov_cpu_leg, &format!("{:.1}%", s.cpu_pct));
                set_label_text(ov_mem_leg, &format!("{:.1}% ({} MB)", mem_pct100, s.mem.used_mb));
                set_label_text(ov_dsk_leg, &format!("R {:.1} / W {:.1} MB/s", s.disk_read_mbs, s.disk_write_mbs));
                set_label_text(ov_net_leg, &format!("↓ {} / ↑ {}", fmt_rate_kbs(s.net_rx_kbs), fmt_rate_kbs(s.net_tx_kbs)));

                // System telemetry strip
                set_label_text(sys_load_lbl, &load_str);
                set_label_text(
                    sys_proc_lbl,
                    &format!("{} total ({} running, {} sleeping)", s.total_procs, s.running_procs, s.sleeping_procs),
                );
                set_label_text(
                    sys_swap_lbl,
                    &format!("{} MB / {} MB", s.mem.swap_used_mb, s.mem.swap_total_mb),
                );

                // CPU tab
                let idle_pct = (100.0 - s.cpu_pct - s.cpu_iowait_pct).max(0.0);
                set_label_text(cpu_big_leg, &format!("Total: {:.1}%", s.cpu_pct));
                set_label_text(
                    cpu_break_lbl,
                    &format!(
                        "User: {:.1}%   •   System: {:.1}%   •   I/O Wait: {:.1}%   •   Idle: {:.1}%",
                        s.cpu_user_pct, s.cpu_sys_pct, s.cpu_iowait_pct, idle_pct
                    ),
                );
                for (i, (pct_l, bar)) in core_widgets.iter().enumerate() {
                    let cp = s.core_pcts.get(i).copied().unwrap_or(0.0);
                    set_label_text(*pct_l, &format!("{:.1}%", cp));
                    gtk_progress_bar_set_fraction(*bar, (cp / 100.0).clamp(0.0, 1.0));
                }

                // Memory tab
                set_label_text(mem_big_leg, &format!("{:.1}% ({} / {} MB)", mem_pct100, s.mem.used_mb, s.mem.total_mb));
                gtk_progress_bar_set_fraction(mem_big_bar, s.mem.pct.clamp(0.0, 1.0));
                set_label_text(m_used_lbl, &format!("{} MB ({:.1}%)", s.mem.used_mb, mem_pct100));
                set_label_text(m_avail_lbl, &format!("{} MB", s.mem.avail_mb));
                set_label_text(m_cache_lbl, &format!("{} MB", s.mem.cached_mb));
                set_label_text(m_buf_lbl, &format!("{} MB", s.mem.buffers_mb));
                set_label_text(m_tot_lbl, &format!("{} MB", s.mem.total_mb));
                set_label_text(m_swp_lbl, &format!("{} MB / {} MB", s.mem.swap_used_mb, s.mem.swap_total_mb));

                // Disk & Network tab legends
                set_label_text(dsk_big_leg, &format!("Read: {:.2} MB/s  •  Write: {:.2} MB/s", s.disk_read_mbs, s.disk_write_mbs));
                set_label_text(net_big_leg, &format!("↓ RX: {}  •  ↑ TX: {}", fmt_rate_kbs(s.net_rx_kbs), fmt_rate_kbs(s.net_tx_kbs)));

                // Update visible tab lists efficiently
                let cur_tab = at.borrow().clone();
                if cur_tab == "cpu" {
                    let mut top = s.procs.clone();
                    top.sort_by(|a, b| b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap_or(std::cmp::Ordering::Equal));
                    clear_box_children(top_cpu_list);
                    for (idx, p) in top.into_iter().take(8).enumerate() {
                        let r = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[if idx % 2 == 0 { "act-proc-row" } else { "act-proc-row-alt" }]);
                        let nl = new_label(&format!("{}  (PID {})", p.name, p.pid), &["act-proc-name"]);
                        gtk_label_set_xalign(nl, 0.0);
                        gtk_widget_set_hexpand(nl, 1);
                        let ul = new_label(&p.user, &["kv-key"]);
                        let cl = new_label(&format!("{:.1}% CPU", p.cpu_pct), &["act-legend-cpu"]);
                        gtk_box_append(r, nl);
                        gtk_box_append(r, ul);
                        gtk_box_append(r, cl);
                        gtk_box_append(top_cpu_list, r);
                    }
                } else if cur_tab == "memory" {
                    let mut top = s.procs.clone();
                    top.sort_by(|a, b| b.rss_mb.partial_cmp(&a.rss_mb).unwrap_or(std::cmp::Ordering::Equal));
                    clear_box_children(top_mem_list);
                    for (idx, p) in top.into_iter().take(8).enumerate() {
                        let r = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[if idx % 2 == 0 { "act-proc-row" } else { "act-proc-row-alt" }]);
                        let nl = new_label(&format!("{}  (PID {})", p.name, p.pid), &["act-proc-name"]);
                        gtk_label_set_xalign(nl, 0.0);
                        gtk_widget_set_hexpand(nl, 1);
                        let ml = new_label(&format!("{:.1} MB ({:.1}%)", p.rss_mb, p.mem_pct), &["act-legend-mem"]);
                        gtk_box_append(r, nl);
                        gtk_box_append(r, ml);
                        gtk_box_append(top_mem_list, r);
                    }
                } else if cur_tab == "disk" {
                    clear_box_children(mounts_list);
                    for m in &s.mounts {
                        let box_m = new_box(GTK_ORIENTATION_VERTICAL, 4, &["act-proc-row"]);
                        let top = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
                        let ml = new_label(&format!("📁 {}  ({})", m.mount, m.device), &["act-proc-name"]);
                        gtk_label_set_xalign(ml, 0.0);
                        gtk_widget_set_hexpand(ml, 1);
                        let ft = new_label(&m.fstype.to_uppercase(), &["act-badge-ok"]);
                        let sz = new_label(
                            &format!("{:.1} GB / {:.1} GB ({:.0}%)", m.used_gb, m.total_gb, m.pct * 100.0),
                            &["act-proc-mono"],
                        );
                        gtk_box_append(top, ml);
                        gtk_box_append(top, ft);
                        gtk_box_append(top, sz);
                        let bar = gtk_progress_bar_new();
                        add_css(bar, "act-bar-disk");
                        gtk_progress_bar_set_fraction(bar, m.pct);
                        gtk_box_append(box_m, top);
                        gtk_box_append(box_m, bar);
                        gtk_box_append(mounts_list, box_m);
                    }
                } else if cur_tab == "network" {
                    clear_box_children(ifaces_list);
                    for (idx, iface) in s.ifaces.iter().enumerate() {
                        let r = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[if idx % 2 == 0 { "act-proc-row" } else { "act-proc-row-alt" }]);
                        let nl = new_label(&iface.name, &["act-proc-name"]);
                        gtk_widget_set_size_request(nl, 96, -1);
                        gtk_label_set_xalign(nl, 0.0);
                        let st_l = new_label(
                            if iface.is_up { "UP" } else { "DOWN" },
                            if iface.is_up { &["act-state-run"] } else { &["act-state-sleep"] },
                        );
                        let ip_l = new_label(&iface.ipv4, &["act-proc-mono"]);
                        gtk_widget_set_hexpand(ip_l, 1);
                        gtk_label_set_xalign(ip_l, 0.0);
                        let rate_l = new_label(
                            &format!("↓ {}  ↑ {}", fmt_rate_kbs(iface.rx_kbs), fmt_rate_kbs(iface.tx_kbs)),
                            &["act-legend-rx"],
                        );
                        let tot_l = new_label(
                            &format!("Total: {} / {}", fmt_bytes(iface.rx_bytes), fmt_bytes(iface.tx_bytes)),
                            &["kv-key"],
                        );
                        gtk_box_append(r, nl);
                        gtk_box_append(r, st_l);
                        gtk_box_append(r, ip_l);
                        gtk_box_append(r, rate_l);
                        gtk_box_append(r, tot_l);
                        gtk_box_append(ifaces_list, r);
                    }
                }

                drop(s);
                if cur_tab == "processes" {
                    rp();
                }

                // Queue redraws on all live Cairo charts
                gtk_widget_queue_draw(ov_cpu_area);
                gtk_widget_queue_draw(ov_mem_area);
                gtk_widget_queue_draw(ov_dsk_area);
                gtk_widget_queue_draw(ov_net_area);
                gtk_widget_queue_draw(cpu_big_area);
                gtk_widget_queue_draw(mem_big_area);
                gtk_widget_queue_draw(dsk_big_area);
                gtk_widget_queue_draw(net_big_area);
            }
        };

        let alive = Rc::new(Cell::new(true));
        {
            let a = Rc::clone(&alive);
            connect_signal(win, "destroy", move || {
                a.set(false);
            });
        }

        render_all();
        render_procs();

        let p_timer = Rc::clone(&paused);
        timeout_add_sec(1, move || {
            if !alive.get() {
                return false;
            }
            if !p_timer.get() {
                render_all();
            }
            true
        });

        gtk_window_set_child(win, root);
        gtk_window_present(win);
    }
}
