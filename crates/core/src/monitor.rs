//! What Apex Overlays itself uses: CPU and memory of the app and its overlay windows
//! (WebView2 runs each window in child processes), data sent to the windows, and how
//! busy the telemetry engine is. Published once a second on the `monitor` topic, only
//! while the Monitoring section or the performance widget is open.

use crate::hub::Hub;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorOut {
    /// CPU used by the whole app, % of the PC (like Task Manager): the app itself and its
    /// overlay windows.
    pub cpu: f32,
    pub cpu_app: f32,
    pub cpu_windows: f32,
    /// The whole PC's CPU use, %.
    pub system_cpu: f32,
    /// Memory (working set), MB: total, the app itself, overlay windows.
    pub memory_mb: f32,
    pub memory_app_mb: f32,
    pub memory_windows_mb: f32,
    /// Processes counted (the app and its window processes).
    pub processes: u32,
    /// Data sent to overlays and the settings page.
    pub sent_kbps: f32,
    pub messages_per_sec: f32,
    /// Connected overlay windows / pages.
    pub clients: u32,
    /// Telemetry ticks processed per second (iRacing sends 60) and the time each takes.
    pub telemetry_hz: f32,
    pub tick_us: f32,
    /// Share of one CPU core the telemetry engine uses, %.
    pub engine_cpu: f32,
    pub uptime: u64,
    pub cores: u32,
}

/// The process and everything it started (WebView2 browser, renderer and GPU processes).
fn tree(sys: &System, root: Pid) -> HashSet<Pid> {
    let mut set = HashSet::from([root]);
    loop {
        let before = set.len();
        for (pid, p) in sys.processes() {
            // Threads are listed as processes on Linux: they are part of their process already.
            if p.thread_kind().is_none() && p.parent().is_some_and(|parent| set.contains(&parent)) {
                set.insert(*pid);
            }
        }
        if set.len() == before {
            return set;
        }
    }
}

fn cpu_and_memory() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing().with_cpu().with_memory()
}

/// Starts measuring the processes in `pids` that `sys` does not know yet, and returns them.
///
/// On Windows sysinfo only keeps a process's starting point for CPU on its second refresh, at
/// least 200 ms after the first: the reading after that would be the process's whole life
/// against the PC's time since it booted, close to 0%. So a new process is refreshed twice
/// here, and its reading is right from the next refresh on. Until then, the returned
/// processes' CPU is not counted.
fn start_measuring(sys: &mut System, pids: &[Pid]) -> Vec<Pid> {
    let new: Vec<Pid> = pids.iter().filter(|p| sys.process(**p).is_none()).copied().collect();
    if !new.is_empty() {
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&new), true, cpu_and_memory());
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL + Duration::from_millis(20));
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&new), true, cpu_and_memory());
    }
    new
}

fn round(v: f32, digits: i32) -> f32 {
    let m = 10f32.powi(digits);
    (v * m).round() / m
}

pub fn spawn(hub: Arc<Hub>) {
    let _ = std::thread::Builder::new().name("monitor".into()).spawn(move || {
        let Ok(me) = sysinfo::get_current_pid() else { return };
        let started = Instant::now();
        let mut sys = System::new();
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f32;
        let counters = |h: &Hub| {
            let s = &h.stats;
            [s.bytes_sent.load(Ordering::Relaxed), s.messages_sent.load(Ordering::Relaxed), s.ticks.load(Ordering::Relaxed), s.tick_ns.load(Ordering::Relaxed)]
        };
        let mut last = (Instant::now(), counters(&hub));
        let mut warm = false;
        // The app's own processes, found from the whole process list only every few seconds:
        // reading CPU and memory of every process on the PC each second was itself a cost.
        // The list is kept in its own System: refreshing every process there without CPU
        // would reset the CPU readings of ours.
        let mut list = System::new();
        let mut pids: Vec<Pid> = vec![];
        let mut found_at: Option<Instant> = None;
        let mut found_clients = usize::MAX;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let now = (Instant::now(), counters(&hub));
            let dt = now.0.duration_since(last.0).as_secs_f32().max(0.001);
            let d = |i: usize| now.1[i].saturating_sub(last.1[i]) as f32;
            let (bytes, msgs, ticks, tick_ns) = (d(0), d(1), d(2), d(3));
            last = now;
            if !hub.has_subscribers("monitor") {
                warm = false;
                continue;
            }
            // Listing every process is itself a little spike, so it is only done every 30 s, or
            // straight away when an overlay connects or one of ours has ended.
            let clients = hub.clients.load(Ordering::Relaxed);
            let gone = pids.iter().any(|p| sys.process(*p).is_none() && found_at.is_some_and(|t| t.elapsed() > Duration::from_secs(2)));
            if found_at.is_none_or(|t| t.elapsed() > Duration::from_secs(30)) || clients != found_clients || gone {
                // Who is who (parents), without reading every process's CPU and memory.
                list.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
                pids = tree(&list, me).into_iter().collect();
                found_at = Some(Instant::now());
                found_clients = clients;
            }
            let new = start_measuring(&mut sys, &pids);
            sys.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), true, cpu_and_memory());
            sys.refresh_cpu_usage();
            let (mut cpu_app, mut cpu_win, mut mem_app, mut mem_win) = (0.0f32, 0.0f32, 0u64, 0u64);
            for pid in &pids {
                let Some(p) = sys.process(*pid) else { continue };
                let cpu = if new.contains(pid) { 0.0 } else { p.cpu_usage() };
                if *pid == me {
                    cpu_app += cpu;
                    mem_app += p.memory();
                } else {
                    cpu_win += cpu;
                    mem_win += p.memory();
                }
            }
            // CPU use is measured between two refreshes: the first reading after the monitor
            // opens has nothing to compare with.
            if !warm {
                warm = true;
                continue;
            }
            let mb = |b: u64| round(b as f32 / 1_048_576.0, 1);
            let out = MonitorOut {
                cpu: round((cpu_app + cpu_win) / cores, 1),
                cpu_app: round(cpu_app / cores, 1),
                cpu_windows: round(cpu_win / cores, 1),
                system_cpu: round(sys.global_cpu_usage(), 1),
                memory_mb: mb(mem_app + mem_win),
                memory_app_mb: mb(mem_app),
                memory_windows_mb: mb(mem_win),
                processes: pids.len() as u32,
                sent_kbps: round(bytes / 1024.0 / dt, 1),
                messages_per_sec: round(msgs / dt, 0),
                clients: hub.clients.load(Ordering::Relaxed) as u32,
                telemetry_hz: round(ticks / dt, 1),
                tick_us: if ticks > 0.0 { round(tick_ns / ticks / 1000.0, 1) } else { 0.0 },
                engine_cpu: round(tick_ns / 1e9 / dt * 100.0, 2),
                uptime: started.elapsed().as_secs(),
                cores: cores as u32,
            };
            hub.publish("monitor", &serde_json::to_string(&out).unwrap_or_default());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_this_process() {
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());
        let me = sysinfo::get_current_pid().unwrap();
        let t = tree(&sys, me);
        assert!(t.contains(&me));
        assert!(sys.process(me).is_some_and(|p| p.memory() > 0));
    }

    #[test]
    fn measures_cpu_reading_only_its_own_processes() {
        // As the monitor does: the process list without CPU or memory, then only ours with them.
        let (mut sys, mut list) = (System::new(), System::new());
        let me = sysinfo::get_current_pid().unwrap();
        list.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        let pids: Vec<Pid> = tree(&list, me).into_iter().collect();
        assert!(start_measuring(&mut sys, &pids).contains(&me), "new processes start being measured");
        assert!(start_measuring(&mut sys, &pids).is_empty(), "known ones are not started again");
        sys.refresh_cpu_usage();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let busy = {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let mut x = 0u64;
                while !stop.load(Ordering::Relaxed) {
                    x = std::hint::black_box(x.wrapping_mul(31).wrapping_add(7));
                }
            })
        };
        std::thread::sleep(Duration::from_millis(400));
        // The process list is read again every few seconds: that must not lose the reading.
        list.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), true, cpu_and_memory());
        sys.refresh_cpu_usage();
        stop.store(true, Ordering::Relaxed);
        busy.join().unwrap();
        // The first reading after the monitor starts is the busy thread's (about one core,
        // 100%), not the process's whole life against the time since the PC booted (~0%).
        let cpu = sys.process(me).map(|p| p.cpu_usage()).unwrap_or(0.0);
        assert!(cpu > 20.0, "busy thread measured at {cpu}%");
        assert!(sys.process(me).is_some_and(|p| p.memory() > 0));
    }
}
