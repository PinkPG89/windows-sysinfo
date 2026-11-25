# Sysmon Tray

작업 표시줄 트레이에 **CPU / RAM 사용률을 숫자 아이콘**으로 보여 주는 가벼운 Windows 모니터링 프로그램입니다.
Rust로 작성되었고, 설치 용량은 약 0.5MB입니다.

*A tiny Windows tray app that shows CPU and RAM usage as number icons in the taskbar.*

## 특징

- CPU(초록), RAM(파랑) 아이콘을 각각 따로 표시, 숫자는 사용률(%)
- 50% 이상 주황, 80% 이상 빨강으로 변경
- 마우스를 올리면 상세 값 툴팁 (RAM은 사용량/전체 GB)
- 1초마다 갱신
- 작업 표시줄에 항상 보이도록 자동 고정 (Windows 11)
- 관리자 권한 없이 설치, 로그인 시 자동 실행

## 설치

[Releases](../../releases)에서 `SysmonTray-Setup.exe`를 받아 실행하세요. 관리자 권한이 필요 없습니다.
자세한 내용은 [설치 설명서](docs/설치%20설명서.html)를 참고하세요.

> 코드 서명이 없어서 Windows SmartScreen 경고가 뜰 수 있습니다. **추가 정보 → 실행**을 누르면 됩니다.

제거: 설정 → 앱 → 설치된 앱 → "Sysmon Tray" 제거

## 직접 빌드

필요한 것: [Rust](https://rustup.rs) (stable), Visual Studio Build Tools (C++ 워크로드),
[Inno Setup 6](https://jrsoftware.org/isinfo.php) (`winget install JRSoftware.InnoSetup`, 설치 파일 생성용)

```powershell
# 실행 파일만 빌드
cargo build --release          # target\release\sysmon-tray.exe

# 설치 파일(Setup.exe)까지 생성
powershell -ExecutionPolicy Bypass -File .\build-dist.ps1   # dist\SysmonTray-Setup.exe
```

## 구조

| 경로 | 설명 |
|---|---|
| `src/main.rs` | 트레이 아이콘 2개 생성, 1초마다 갱신, 레지스트리로 작업 표시줄 고정 |
| `installer/` | Inno Setup 스크립트(`sysmon-tray.iss`) |
| `docs/` | 설치 설명서 |
| `build-dist.ps1` | 빌드 + `SysmonTray-Setup.exe` 생성 |
| `.cargo/config.toml` | VC++ 런타임을 exe에 포함 (별도 설치 불필요) |

사용한 주요 크레이트: [`tray-icon`](https://crates.io/crates/tray-icon), [`tao`](https://crates.io/crates/tao), [`sysinfo`](https://crates.io/crates/sysinfo), [`winreg`](https://crates.io/crates/winreg)

## 라이선스

[MIT](LICENSE)
