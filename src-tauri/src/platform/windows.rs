// Windows 平台后端
//
// 阶段 5：EnumDisplayMonitors 枚举所有显示器
// 阶段 8：WM_POWERBROADCAST 电源事件（待实现）

use super::{MonitorApi, MonitorRect};
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HMONITOR, HDC, MONITORINFO,
};
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

pub struct WindowsMonitorApi;

impl MonitorApi for WindowsMonitorApi {
    fn list_monitors(&self) -> Vec<MonitorRect> {
        enumerate_monitors()
    }
}

/// 安全封装：调用 EnumDisplayMonitors，收集每个显示器矩形
fn enumerate_monitors() -> Vec<MonitorRect> {
    let mut results: Vec<MonitorRect> = Vec::new();
    let results_ptr = &mut results as *mut Vec<MonitorRect>;

    // SAFETY: EnumDisplayMonitors 回调里只读取 HMONITOR 信息并 push 到 results。
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(enum_proc), LPARAM(results_ptr as isize));
    }
    results
}

/// EnumDisplayMonitors 回调
unsafe extern "system" fn enum_proc(
    hmon: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let results = &mut *(lparam.0 as *mut Vec<MonitorRect>);

    let mut info: MONITORINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let ok = GetMonitorInfoW(hmon, &mut info);
    if ok.as_bool() {
        let r = info.rcMonitor;
        results.push(MonitorRect {
            x: r.left,
            y: r.top,
            width: r.right - r.left,
            height: r.bottom - r.top,
            is_primary: (info.dwFlags & MONITORINFOF_PRIMARY) != 0,
        });
    }
    BOOL(1) // 继续枚举
}
