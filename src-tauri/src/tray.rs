// tray.rs — 系统托盘
//
// 阶段 3：
// - 三状态图标：working(蓝) / resting(绿) / paused(灰)
// - 悬停 tooltip 每秒刷新：工作中/休息中/已暂停 + 剩余时间
// - 右键菜单：查看状态、暂停/恢复、重置计时、打开配置、退出
// - 双击托盘打开配置窗口
// - macOS：menu bar item 行为适配（左键直接展开菜单，menuOnLeftClick:false 已在 conf 配置）
//
// 图标用 include_bytes! 编译进二进制，避免运行时路径依赖。

use crate::engine::EngineHandle;
use crate::state::Status;
use crate::tray_icons::{TRAY_PAUSED_RGBA, TRAY_RESTING_RGBA, TRAY_WORKING_RGBA};
use tauri::{
    image::Image,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

/// 托盘菜单 item id
const MENU_PAUSE_RESUME: &str = "pause_resume";
const MENU_RESET: &str = "reset";
const MENU_SETTINGS: &str = "settings";
const MENU_LANGUAGE: &str = "language";
const MENU_LANGUAGE_ZH: &str = "language_zh";
const MENU_LANGUAGE_EN: &str = "language_en";
const MENU_QUIT: &str = "quit";

/// 三状态图标 RGBA 数据（32x32，编译期内嵌）
fn icon_for_status(status: Status) -> &'static [u8] {
    match status {
        Status::Running => TRAY_WORKING_RGBA,
        Status::Resting => TRAY_RESTING_RGBA,
        Status::Paused => TRAY_PAUSED_RGBA,
    }
}

fn make_image(status: Status) -> Image<'static> {
    Image::new(icon_for_status(status), 32, 32)
}

/// 状态中文文案
fn status_label(status: Status) -> &'static str {
    match status {
        Status::Running => "工作中",
        Status::Resting => "休息中",
        Status::Paused => "已暂停",
    }
}

/// 格式化剩余时间为 MM:SS
fn fmt_remaining(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// 创建托盘。在 setup 阶段调用。
pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = Menu::with_items(
        app,
        &[
            // 查看状态（仅展示，不可点）
            &MenuItem::with_id(app, "status_display", "FocusLock", false, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, MENU_PAUSE_RESUME, "暂停 / 恢复", true, None::<&str>)?,
            &MenuItem::with_id(app, MENU_RESET, "重置计时", true, None::<&str>)?,
            &MenuItem::with_id(app, MENU_SETTINGS, "打开配置…", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?,
        ],
    )?;

    let icon = make_image(Status::Running);
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("FocusLock · 启动中…")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event)
        .build(app)?;
    Ok(())
}

/// 菜单点击处理
fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let id = event.id().as_ref();
    tracing::info!("托盘菜单点击: {}", id);
    let app = app.clone();
    match id {
        MENU_PAUSE_RESUME => {
            let engine = match app.try_state::<EngineHandle>() {
                Some(e) => e.inner().clone(),
                None => return,
            };
            tauri::async_runtime::spawn(async move {
                let (status, _, _) = engine.get_status().await;
                let now = chrono::Utc::now().timestamp();
                if status == Status::Paused {
                    engine.resume(now).await;
                } else {
                    engine.pause(now).await;
                }
            });
        }
        MENU_RESET => {
            let engine = match app.try_state::<EngineHandle>() {
                Some(e) => e.inner().clone(),
                None => return,
            };
            tauri::async_runtime::spawn(async move {
                let (config, _) = crate::config::Config::load();
                let now = chrono::Utc::now().timestamp();
                engine.reset_to_first(config, now).await;
            });
        }
        MENU_SETTINGS => {
            show_settings_window(&app);
        }
        MENU_QUIT => {
            tracing::info!("用户选择退出，应用结束。");
            app.exit(0);
        }
        _ => {}
    }
}

/// 托盘图标交互：左键单击切换暂停/恢复
fn on_tray_icon_event<R: Runtime>(
    tray: &tauri::tray::TrayIcon<R>,
    event: TrayIconEvent,
) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        // 左键单击切换暂停/恢复
        let app = tray.app_handle().clone();
        let engine = match app.try_state::<EngineHandle>() {
            Some(e) => e.inner().clone(),
            None => return,
        };
        tauri::async_runtime::spawn(async move {
            let (status, _, _) = engine.get_status().await;
            let now = chrono::Utc::now().timestamp();
            if status == Status::Paused {
                engine.resume(now).await;
            } else {
                engine.pause(now).await;
            }
        });
    }
}

/// 显示配置窗口（单例：已打开则聚焦）
fn show_settings_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    } else {
        tracing::warn!("主窗口未找到，无法打开配置界面");
    }
}

/// 启动一个后台 task：每秒读取引擎状态，刷新托盘 tooltip 和图标。
/// 由 lib.rs 在引擎启动后调用。
pub fn spawn_tray_updater<R: Runtime>(app: AppHandle<R>, engine: EngineHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_status: Option<Status> = None;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let (status, _idx, remaining) = engine.get_status().await;

            // 刷新 tooltip
            let tooltip = format!(
                "FocusLock · {}，剩余 {}",
                status_label(status),
                fmt_remaining(remaining)
            );
            if let Some(tray) = app.tray_by_id("main") {
                let _ = tray.set_tooltip(Some(&tooltip));
                // 状态变化时切换图标
                if last_status != Some(status) {
                    let img = make_image(status);
                    let _ = tray.set_icon(Some(img));
                    last_status = Some(status);
                }
            }
        }
    });
}
