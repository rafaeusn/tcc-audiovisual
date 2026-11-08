//! Medições iguais para os dois protótipos: tempo até o primeiro quadro (desde a criação do processo), fps,
//! tempos de quadro (p50/p95/p99), memória (working set e privada) e CPU média do processo.

use serde::Serialize;
use std::time::Instant;

#[derive(Default)]
pub struct FrameClock {
    last: Option<Instant>,
    frames: u64,
    first_frame_ms: Option<f64>,
    times_ms: Vec<f32>,
    window_frames: u32,
    window_start: Option<Instant>,
    pub fps: f32,
    bench_start: Option<(Instant, f64)>,
}

impl FrameClock {
    /// Chamar uma vez por quadro desenhado. Devolve o dt em segundos.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        if self.first_frame_ms.is_none() {
            self.first_frame_ms = Some(process_age_ms());
        }
        let dt = self.last.map(|l| (now - l).as_secs_f32()).unwrap_or(1.0 / 60.0);
        self.last = Some(now);
        self.frames += 1;
        if self.bench_start.is_some() && self.frames > 1 {
            self.times_ms.push(dt * 1000.0);
        }
        let ws = *self.window_start.get_or_insert(now);
        self.window_frames += 1;
        let el = (now - ws).as_secs_f32();
        if el >= 0.5 {
            self.fps = self.window_frames as f32 / el;
            self.window_frames = 0;
            self.window_start = Some(now);
        }
        dt.min(0.1)
    }

    /// Começa a contar para o relatório (depois do aquecimento).
    pub fn start_bench(&mut self) {
        self.times_ms.clear();
        self.bench_start = Some((Instant::now(), cpu_ms()));
    }

    pub fn report(&self, framework: &str, renderer: &str, audio: &str, window: (u32, u32)) -> Report {
        let (start, cpu0) = self.bench_start.unwrap_or((Instant::now(), cpu_ms()));
        let secs = start.elapsed().as_secs_f64().max(1e-3);
        let mut t = self.times_ms.clone();
        t.sort_by(|a, b| a.total_cmp(b));
        let pct = |p: f64| if t.is_empty() { 0.0 } else { t[((t.len() - 1) as f64 * p).round() as usize] };
        let (ws, private) = memory_mb();
        Report {
            framework: framework.into(),
            renderer: renderer.into(),
            audio: audio.into(),
            window: format!("{}x{}", window.0, window.1),
            startup_ms_first_frame: self.first_frame_ms.unwrap_or(0.0),
            duration_s: secs,
            frames: t.len(),
            avg_fps: t.len() as f64 / secs,
            frame_ms_p50: pct(0.5),
            frame_ms_p95: pct(0.95),
            frame_ms_p99: pct(0.99),
            frame_ms_max: t.last().copied().unwrap_or(0.0),
            working_set_mb: ws,
            private_mb: private,
            cpu_percent: (cpu_ms() - cpu0) / (secs * 1000.0) * 100.0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub framework: String,
    pub renderer: String,
    pub audio: String,
    pub window: String,
    pub startup_ms_first_frame: f64,
    pub duration_s: f64,
    pub frames: usize,
    pub avg_fps: f64,
    pub frame_ms_p50: f32,
    pub frame_ms_p95: f32,
    pub frame_ms_p99: f32,
    pub frame_ms_max: f32,
    pub working_set_mb: f64,
    pub private_mb: f64,
    /// CPU do processo (100% = um núcleo inteiro).
    pub cpu_percent: f64,
}

impl Report {
    pub fn save(&self, path: &str) {
        let _ = std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap_or(std::path::Path::new(".")));
        let _ = std::fs::write(path, serde_json::to_string_pretty(self).unwrap_or_default());
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

/// Argumentos comuns: `--bench <segundos>`, `--out <arquivo.json>`, `--synthetic`.
pub struct BenchArgs {
    pub seconds: Option<f32>,
    pub out: String,
    pub synthetic: bool,
}

impl BenchArgs {
    pub fn parse(default_out: &str) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
        Self {
            seconds: get("--bench").and_then(|s| s.parse().ok()),
            out: get("--out").unwrap_or_else(|| default_out.into()),
            synthetic: args.iter().any(|a| a == "--synthetic"),
        }
    }
}

#[cfg(windows)]
fn filetime_ms(ft: windows_sys::Win32::Foundation::FILETIME) -> f64 {
    (((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64) as f64 / 10_000.0
}

#[cfg(windows)]
fn times() -> (f64, f64) {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let z = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut c, mut e, mut k, mut u) = (z, z, z, z);
    unsafe { GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u) };
    (filetime_ms(c), filetime_ms(k) + filetime_ms(u))
}

#[cfg(windows)]
fn now_filetime_ms() -> f64 {
    let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    // FILETIME conta desde 1601; a diferença para 1970 é 11644473600 s.
    since.as_secs_f64() * 1000.0 + 11_644_473_600_000.0
}

/// Milissegundos desde que o processo foi criado (inclui carregar o executável e as DLLs).
pub fn process_age_ms() -> f64 {
    #[cfg(windows)]
    {
        now_filetime_ms() - times().0
    }
    #[cfg(not(windows))]
    {
        0.0
    }
}

fn cpu_ms() -> f64 {
    #[cfg(windows)]
    {
        times().1
    }
    #[cfg(not(windows))]
    {
        0.0
    }
}

pub fn memory_mb() -> (f64, f64) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        let mut c: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
        c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut c as *mut _ as *mut _, c.cb) };
        if ok != 0 {
            return (c.WorkingSetSize as f64 / 1048576.0, c.PrivateUsage as f64 / 1048576.0);
        }
        (0.0, 0.0)
    }
    #[cfg(not(windows))]
    {
        (0.0, 0.0)
    }
}

/// Hora local "HH:MM:SS" (sem depender de crate de fuso horário).
pub fn local_clock() -> String {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut t = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut t) };
        format!("{:02}:{:02}:{:02}", t.wHour, t.wMinute, t.wSecond)
    }
    #[cfg(not(windows))]
    {
        String::from("--:--:--")
    }
}
