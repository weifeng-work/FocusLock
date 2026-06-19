// shortcut.rs — 全局快捷键（跳过休息）
//
// 阶段 7：
// - 注册 config.skip_shortcut（默认 CmdOrCtrl+Shift+F2）
// - 仅在 Resting 状态下生效；按下 → 调用 engine.skip_rest
// - 注册失败（快捷键被占用）→ 日志 + 通知提示用户
// - 配置变更后需重新注册（通过 update 命令）

use crate::engine::EngineHandle;
use crate::state::Status;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// 注册跳过休息快捷键。在 setup 阶段调用。
pub fn register<R: Runtime>(app: &AppHandle<R>, accelerator: &str, engine: EngineHandle) {
    let shortcut: Shortcut = match accelerator.parse() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("快捷键解析失败 [{}]: {}", accelerator, e);
            return;
        }
    };

    let app_clone = app.clone();
    let result = app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
        // 仅在按下时触发（Released 忽略）
        if event.state() != ShortcutState::Pressed {
            return;
        }
        let engine = engine.clone();
        tauri::async_runtime::spawn(async move {
            let (status, _, _) = engine.get_status().await;
            if status != Status::Resting {
                tracing::debug!("快捷键触发但当前非 Resting，忽略");
                return;
            }
            let now = chrono::Utc::now().timestamp();
            if engine.skip_rest(now).await {
                tracing::info!("快捷键跳过休息成功");
            }
        });
        let _ = &app_clone;
    });

    if let Err(e) = result {
        tracing::error!("快捷键注册失败 [{}]：{}。可能被其他程序占用。", accelerator, e);
        let title = "FocusLock · 快捷键注册失败";
        let body = format!(
            "「{}」可能被其他程序占用，跳过休息功能不可用。可在配置中修改快捷键。",
            accelerator
        );
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            use tauri_plugin_notification::NotificationExt;
            let _ = app2.notification().builder().title(title).body(&body).show();
        });
    } else {
        tracing::info!("快捷键已注册：{}", accelerator);
    }
}

/// 重新注册快捷键（配置变更后调用）
pub fn update<R: Runtime>(
    app: &AppHandle<R>,
    old_accelerator: &str,
    new_accelerator: &str,
    engine: EngineHandle,
) {
    // 先注销旧的
    if let Ok(old) = old_accelerator.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(old);
    }
    register(app, new_accelerator, engine);
}
