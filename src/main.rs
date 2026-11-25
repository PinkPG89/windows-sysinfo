#![windows_subsystem = "windows"] // 콘솔 창 숨김

use std::time::{Duration, Instant};

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, SYSTEMTIME};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::System::Threading::CreateMutexW;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

type Rgba = [u8; 4];

// ---- 설정 값 ----
const INTERVAL: Duration = Duration::from_secs(1);
const CPU_COLOR: Rgba = [30, 140, 70, 255]; // 초록
const RAM_COLOR: Rgba = [40, 100, 200, 255]; // 파랑
const WARN_PERCENT: f32 = 50.0;
const WARN_COLOR: Rgba = [215, 140, 20, 255]; // 주황
const CRIT_PERCENT: f32 = 80.0;
const CRIT_COLOR: Rgba = [200, 40, 40, 255]; // 빨강
const TEXT_COLOR: Rgba = [255, 255, 255, 255];

const ICON_SIZE: u32 = 32;
const TRAY_RETRY_SECS: u32 = 120;
const PIN_MAX_TRIES: u32 = 10;

/// 3x5 비트맵 숫자 폰트 (각 행의 하위 3비트 사용)
const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b001, 0b001, 0b001], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

/// 아이콘에 표시할 두 자리 값 (100%는 99로 표시).
fn display_value(percent: f32) -> u32 {
    (percent.round() as u32).min(99)
}

/// 사용률에 따른 배경색: 평소 `normal`, 경고/위험 기준 이상이면 주황/빨강.
fn level_color(percent: f32, normal: Rgba) -> Rgba {
    if percent >= CRIT_PERCENT {
        CRIT_COLOR
    } else if percent >= WARN_PERCENT {
        WARN_COLOR
    } else {
        normal
    }
}

/// 배경색 위에 두 자리 숫자를 그린 아이콘을 만든다.
fn render_icon(value: u32, bg: Rgba) -> Icon {
    let mut rgba = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&bg);
    }

    // 숫자 2개: 각 3x5 칸을 4배 확대 → 12x20, 간격 4 → 총 폭 28
    let scale = 4u32;
    let (gw, gh) = (3 * scale, 5 * scale);
    let gap = 4u32;
    let x0 = (ICON_SIZE - (gw * 2 + gap)) / 2;
    let y0 = (ICON_SIZE - gh) / 2;
    for (n, digit) in [value / 10 % 10, value % 10].into_iter().enumerate() {
        let gx = x0 + n as u32 * (gw + gap);
        for row in 0..5u32 {
            for col in 0..3u32 {
                if DIGITS[digit as usize][row as usize] >> (2 - col) & 1 == 1 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = gx + col * scale + dx;
                            let y = y0 + row * scale + dy;
                            let i = ((y * ICON_SIZE + x) * 4) as usize;
                            rgba[i..i + 4].copy_from_slice(&TEXT_COLOR);
                        }
                    }
                }
            }
        }
    }
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("icon buffer has a fixed valid size")
}

/// 트레이 아이콘 하나(CPU 또는 RAM). 표시 내용이 바뀔 때만 아이콘/툴팁을 갱신한다.
struct Gauge {
    tray: TrayIcon,
    normal: Rgba,
    shown: Option<(u32, Rgba)>,
    tooltip: String,
}

impl Gauge {
    /// 로그인 직후에는 작업 표시줄이 아직 준비되지 않아 생성이 실패할 수 있으므로
    /// 1초 간격으로 재시도한다.
    fn new(menu: &Menu, label: &str, normal: Rgba) -> Option<Self> {
        let tooltip = format!("{label} 측정 중...");
        for attempt in 1..=TRAY_RETRY_SECS {
            match TrayIconBuilder::new()
                .with_menu(Box::new(menu.clone()))
                .with_tooltip(&tooltip)
                .with_icon(render_icon(0, normal))
                .build()
            {
                Ok(tray) => {
                    return Some(Self {
                        tray,
                        normal,
                        shown: Some((0, normal)),
                        tooltip,
                    })
                }
                Err(e) => {
                    if attempt == 1 || attempt % 30 == 0 {
                        log_error(&format!("{label} tray build failed (attempt {attempt}): {e}"));
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }
        None
    }

    fn update(&mut self, percent: f32, tooltip: String) {
        let next = (display_value(percent), level_color(percent, self.normal));
        if self.shown != Some(next) && self.tray.set_icon(Some(render_icon(next.0, next.1))).is_ok()
        {
            self.shown = Some(next);
        }
        if self.tooltip != tooltip && self.tray.set_tooltip(Some(&tooltip)).is_ok() {
            self.tooltip = tooltip;
        }
    }
}

/// Windows 11: 트레이 아이콘을 "숨겨진 아이콘(^)"이 아니라 작업 표시줄에 항상 보이게 승격한다.
/// 아이콘이 처음 등록되면 HKCU\Control Panel\NotifyIconSettings\<id> 키가 생기고,
/// 거기 IsPromoted=1 을 쓰면 고정된다. `expected`개 이상 고정되면 true.
fn pin_tray_icons(expected: usize) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let exe = exe.to_string_lossy().to_lowercase();

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(root) = hkcu.open_subkey_with_flags("Control Panel\\NotifyIconSettings", KEY_READ)
    else {
        return false; // Windows 10 등 해당 키가 없는 환경
    };

    let mut promoted_count = 0;
    for name in root.enum_keys().flatten() {
        let Ok(key) = root.open_subkey_with_flags(&name, KEY_READ | KEY_WRITE) else {
            continue;
        };
        let Ok(path) = key.get_value::<String, _>("ExecutablePath") else {
            continue;
        };
        if path.to_lowercase() == exe {
            let promoted: u32 = key.get_value("IsPromoted").unwrap_or(0);
            if promoted == 1 || key.set_value("IsPromoted", &1u32).is_ok() {
                promoted_count += 1;
            }
        }
    }
    promoted_count >= expected
}

/// 이미 실행 중인 인스턴스가 있으면 true. 뮤텍스 핸들은 프로세스 종료 때까지 유지한다.
fn already_running() -> bool {
    let name: Vec<u16> = "Local\\SysmonTray.SingleInstance\0".encode_utf16().collect();
    // SAFETY: name은 널로 끝나는 유효한 UTF-16 문자열이고 호출 동안 살아 있다.
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        !handle.is_null() && GetLastError() == ERROR_ALREADY_EXISTS
    }
}

/// 오류를 %LOCALAPPDATA%\SysmonTray\error.log 에 남긴다. 정상 동작 중에는 쓰지 않는다.
/// 콘솔 없는 앱이라 패닉/오류가 나면 흔적 없이 종료되기 때문에 필요하다.
fn log_error(msg: &str) {
    use std::io::Write;
    let Some(base) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let dir = std::path::Path::new(&base).join("SysmonTray");
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("error.log"))
    {
        // SAFETY: SYSTEMTIME은 정수 필드만 가진 구조체라 0으로 초기화해도 유효하다.
        let t = unsafe {
            let mut t: SYSTEMTIME = std::mem::zeroed();
            GetLocalTime(&mut t);
            t
        };
        let _ = writeln!(
            f,
            "[{:04}-{:02}-{:02} {:02}:{:02}:{:02}] {}",
            t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, msg
        );
    }
}

fn main() {
    std::panic::set_hook(Box::new(|info| log_error(&format!("panic: {info}"))));

    if already_running() {
        return;
    }

    let event_loop = EventLoopBuilder::new().build();

    let quit = MenuItem::new("종료", true, None);
    let menu = Menu::new();
    if let Err(e) = menu.append(&quit) {
        log_error(&format!("menu append failed: {e}"));
    }

    let (Some(mut cpu), Some(mut ram)) = (
        Gauge::new(&menu, "CPU", CPU_COLOR),
        Gauge::new(&menu, "RAM", RAM_COLOR),
    ) else {
        log_error("giving up: could not create tray icons");
        return;
    };

    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    );

    // 레지스트리 항목은 아이콘 등록 직후 생기므로, 성공할 때까지 몇 번 재시도한다.
    let mut pinned = false;
    let mut pin_tries = 0;

    // 다음 갱신 시각을 따로 들고 있어야, 다른 이벤트가 계속 들어와도 갱신이 밀리지 않는다.
    let mut next_tick = Instant::now();

    event_loop.run(move |_event, _, control_flow| {
        let now = Instant::now();
        if now >= next_tick {
            next_tick = now + INTERVAL;

            sys.refresh_cpu_usage();
            sys.refresh_memory();

            const GB: f64 = 1024.0 * 1024.0 * 1024.0;
            let cpu_pct = sys.global_cpu_usage();
            let used = sys.used_memory() as f64 / GB;
            let total = sys.total_memory() as f64 / GB;
            let ram_pct = if total > 0.0 { (used / total * 100.0) as f32 } else { 0.0 };

            cpu.update(cpu_pct, format!("CPU {cpu_pct:.0}%"));
            ram.update(
                ram_pct,
                format!("RAM {ram_pct:.0}%\n{used:.1} / {total:.1} GB"),
            );

            if !pinned && pin_tries < PIN_MAX_TRIES {
                pin_tries += 1;
                pinned = pin_tray_icons(2);
            }
        }
        *control_flow = ControlFlow::WaitUntil(next_tick);

        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == quit.id() {
                *control_flow = ControlFlow::Exit;
            }
        }
    });
}
