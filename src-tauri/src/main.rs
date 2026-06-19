// FocusLock 应用入口
// 阶段 0：最小可运行——单实例锁 + 后台启动（默认隐藏窗口）+ 托盘占位
// 后续阶段将在此挂载：config/state 模块、计时引擎、平台抽象层、托盘、通知、遮罩、快捷键

// Windows：通过独立 bundle identifier (com.focuslock.app) 使 WebView2 用户数据目录
// 与本机其他 Tauri 应用自然隔离，避免同时 dev 时 WebView2 环境创建冲突。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    focuslock_lib::run()
}
