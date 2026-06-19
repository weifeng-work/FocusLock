// notify.rs — 系统通知封装
//
// 阶段 4：
// - 工作开始 / 准备休息 / 休息开始 / 跳过休息 / 重置 / 自动恢复 各发一条原生通知
// - macOS：首次发通知时系统会自动弹授权对话框；授权状态由系统管理。
//   若用户拒绝，通知静默失败，降级路径：托盘 tooltip 已每秒刷新（tray.rs），
//   配置界面顶部红点提示在阶段 6 实现。
//
// 关于「准备休息通知 + 暂停按钮」：
//   tauri-plugin-notification 2.x 桌面端 NotificationBuilder 不支持 action/button
//   （仅 title/body/icon/sound）。原计划风险表已记录此备选：
//   「暂停循环」通过托盘菜单「暂停 / 恢复」完成（tray.rs 已实现）。
//   准备休息通知文案中提示用户「右键托盘暂停」。

use crate::config::RestReminderMode;
use crate::engine::EngineEvent;
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

/// 发送一条通知。失败仅记日志，不阻断引擎。
fn send<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    let result = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
    if let Err(e) = result {
        tracing::warn!("通知发送失败（可能未授权）: {} - {}", title, e);
    }
}

/// 格式化剩余秒数为 "X 分 Y 秒" 或 "Y 秒"
fn fmt_remaining(secs: u64) -> String {
    if secs >= 60 {
        format!("{} 分 {} 秒", secs / 60, secs % 60)
    } else {
        format!("{} 秒", secs)
    }
}

/// 处理引擎事件，发送对应通知。
/// 在 lib.rs 的事件桥接 task 中调用。
pub fn on_engine_event<R: Runtime>(app: &AppHandle<R>, event: &EngineEvent) {
    match event {
        EngineEvent::WorkStarted { remaining } => {
            send(
                app,
                "FocusLock · 开始工作",
                &format!("进入工作阶段，剩余 {}", fmt_remaining(*remaining)),
            );
        }
        EngineEvent::PrepareRest { remaining } => {
            send(
                app,
                "FocusLock · 即将休息",
                &format!(
                    "剩余 {}，请保存工作。如需暂停循环，请右键托盘选「暂停 / 恢复」。",
                    fmt_remaining(*remaining)
                ),
            );
        }
        EngineEvent::RestStarted { remaining, mode } => {
            let mode_text = match mode {
                RestReminderMode::Fullscreen => "全屏遮罩",
                RestReminderMode::Popup => "弹窗",
            };
            send(
                app,
                "FocusLock · 开始休息",
                &format!(
                    "{}模式，剩余 {}。按快捷键可跳过休息。",
                    mode_text,
                    fmt_remaining(*remaining)
                ),
            );
        }
        EngineEvent::RestEnded => {
            // 休息自然结束 → 紧接着会发 WorkStarted/RestStarted，这里不重复通知
        }
        EngineEvent::RestSkipped => {
            send(app, "FocusLock · 已跳过休息", "进入下一个工作阶段。");
        }
        EngineEvent::StatusChanged { status, remaining } => {
            // 状态变更通知由 WorkStarted/RestSkipped 等具体事件覆盖，
            // 这里只在「暂停 / 恢复」时补充提示。
            use crate::state::Status;
            match status {
                Status::Paused => {
                    send(app, "FocusLock · 已暂停", "倒计时已冻结，右键托盘可恢复。");
                }
                Status::Running => {
                    // 恢复（非首次启动的 WorkStarted）
                    send(
                        app,
                        "FocusLock · 已恢复",
                        &format!("继续倒计时，剩余 {}", fmt_remaining(*remaining)),
                    );
                }
                Status::Resting => {}
            }
        }
        EngineEvent::ResetDueToInactivity => {
            send(
                app,
                "FocusLock · 检测到长时间离开",
                "已重置计时，从第一阶段重新开始。",
            );
        }
    }
}

/// macOS 首启触发授权：发一条静默的「启动」通知。
/// 系统会因此弹出通知授权对话框，用户授权后后续通知才能正常显示。
/// 拒绝则此通知不显示，但应用其他功能不受影响。
#[cfg(target_os = "macos")]
pub fn trigger_permission_prompt<R: Runtime>(app: &AppHandle<R>) {
    send(app, "FocusLock 已启动", "专注节奏管理已就绪。");
}

#[cfg(not(target_os = "macos"))]
pub fn trigger_permission_prompt<R: Runtime>(_app: &AppHandle<R>) {
    // Windows 无需主动触发授权
}
