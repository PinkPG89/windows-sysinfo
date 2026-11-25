#![windows_subsystem = "windows"] // 콘솔 창 숨김

use std::time::{Duration, Instant};

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const SIZE: u32 = 32;
const CPU_COLOR: [u8; 4] = [30, 140, 70, 255]; // 초록
const RAM_COLOR: [u8; 4] = [40, 100, 200, 255]; // 파랑
const INTERVAL: Duration = Duration::from_secs(1);

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

/// 사용률(%)을 두 자리 숫자로 그린 아이콘. 평소 색은 `normal`, 50%/80% 이상이면 주황/빨강.
fn make_icon(cpu: f32, normal: [u8; 4]) -> Icon {
    let value = (cpu.round() as u32).min(99);
    let bg: [u8; 4] = match cpu {
        c if c >= 80.0 => [200, 40, 40, 255],
        c if c >= 50.0 => [215, 140, 20, 255],
        _ => normal,
    };
    let fg: [u8; 4] = [255, 255, 255, 255];

    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&bg);
    }

    // 숫자 2개: 각 3x5 칸을 4배 확대 → 12x20, 간격 4 → 총 폭 28
    let scale = 4u32;
    let (gw, gh) = (3 * scale, 5 * scale);
    let gap = 4u32;
    let x0 = (SIZE - (gw * 2 + gap)) / 2;
    let y0 = (SIZE - gh) / 2;
    for (n, digit) in [value / 10, value % 10].into_iter().enumerate() {
        let gx = x0 + n as u32 * (gw + gap);
        for row in 0..5u32 {
            for col in 0..3u32 {
                if DIGITS[digit as usize][row as usize] >> (2 - col) & 1 == 1 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = gx + col * scale + dx;
                            let y = y0 + row * scale + dy;
                            let i = ((y * SIZE + x) * 4) as usize;
                            rgba[i..i + 4].copy_from_slice(&fg);
                        }
                    }
                }
            }
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("invalid icon")
}

/// Windows 11: 트레이 아이콘을 "숨겨진 아이콘(^)"이 아니라 작업 표시줄에 항상 보이게 승격한다.
/// 아이콘이 처음 등록되면 HKCU\Control Panel\NotifyIconSettings\<id> 키가 생기고,
/// 거기 IsPromoted=1 을 쓰면 고정된다. 성공하면 true.
fn pin_tray_icon(expected: usize) -> bool {
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

/// 오류를 %LOCALAPPDATA%\SysmonTray\error.log 에 남긴다.
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
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[unix {}] {}", secs, msg);
    }
}

/// 트레이 아이콘 생성. 로그인 직후에는 작업 표시줄이 아직 준비되지 않아
/// 실패할 수 있으므로 최대 120초(1초 간격) 재시도한다.
fn build_tray(menu: &Menu, tooltip: &str, color: [u8; 4]) -> Option<tray_icon::TrayIcon> {
    for attempt in 1..=120 {
        match TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_tooltip(tooltip)
            .with_icon(make_icon(0.0, color))
            .build()
        {
            Ok(t) => return Some(t),
            Err(e) => {
                if attempt == 1 || attempt % 30 == 0 {
                    log_error(&format!("tray build failed (attempt {attempt}): {e}"));
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
    None
}

fn main() {
    std::panic::set_hook(Box::new(|info| log_error(&format!("panic: {info}"))));

    let event_loop = EventLoopBuilder::new().build();

    let quit = MenuItem::new("종료", true, None);
    let menu = Menu::new();
    if let Err(e) = menu.append(&quit) {
        log_error(&format!("menu append failed: {e}"));
    }

    let (Some(cpu_tray), Some(ram_tray)) = (
        build_tray(&menu, "CPU 측정 중...", CPU_COLOR),
        build_tray(&menu, "RAM 측정 중...", RAM_COLOR),
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

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + INTERVAL);

        if let Event::NewEvents(StartCause::ResumeTimeReached { .. }) = event {
            sys.refresh_cpu_usage();
            sys.refresh_memory();

            let cpu = sys.global_cpu_usage();
            let used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
            let total = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

            let ram_pct = if total > 0.0 { (used / total * 100.0) as f32 } else { 0.0 };

            let _ = cpu_tray.set_icon(Some(make_icon(cpu, CPU_COLOR)));
            let _ = cpu_tray.set_tooltip(Some(format!("CPU {:.0}%", cpu)));
            let _ = ram_tray.set_icon(Some(make_icon(ram_pct, RAM_COLOR)));
            let _ = ram_tray.set_tooltip(Some(format!(
                "RAM {:.0}%\n{:.1} / {:.1} GB",
                ram_pct, used, total
            )));

            if !pinned && pin_tries < 10 {
                pin_tries += 1;
                pinned = pin_tray_icon(2);
            }
        }

        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == quit.id() {
                *control_flow = ControlFlow::Exit;
            }
        }
    });
}
