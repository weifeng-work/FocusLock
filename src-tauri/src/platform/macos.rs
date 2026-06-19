// macOS 平台后端
//
// 阶段 5：NSScreen.screens 枚举所有显示器
// 阶段 8：NSWorkspaceDidWakeNotification 电源事件（待实现）
//
// macOS 实现使用 objc2-app-kit 的 NSScreen API。
// 注意：本文件 #[cfg(target_os = "macos")] 才会编译，Windows 构建时跳过。

use super::{MonitorApi, MonitorRect};

pub struct MacosMonitorApi;

impl MonitorApi for MacosMonitorApi {
    fn list_monitors(&self) -> Vec<MonitorRect> {
        // 阶段 5 实现：访问 NSScreen.screens
        // 由于本机是 Windows 开发环境，macOS 实现先以占位形式存在，
        // 移植到 Mac 后用以下伪代码逻辑填充：
        //
        // use objc2_app_kit::NSScreen;
        // use objc2_foundation::MainThreadMarker;
        // let mt = MainThreadMarker::new().unwrap();
        // let screens = NSScreen::screens(mt);
        // for i in 0..screens.count() {
        //     let screen = screens.get(i).unwrap();
        //     let frame = screen.frame();
        //     let primary = (i == 0);
        //     results.push(MonitorRect {
        //         x: frame.origin.x as i32,
        //         y: frame.origin.y as i32,
        //         width: frame.size.width as i32,
        //         height: frame.size.height as i32,
        //         is_primary: primary,
        //     });
        // }
        Vec::new()
    }
}
