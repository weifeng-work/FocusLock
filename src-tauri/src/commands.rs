// commands.rs — 暴露给前端的 Tauri command
//
// 阶段 3 起接入：前端（配置面板 / 遮罩页）通过 invoke 调用这些命令。
// 引擎句柄通过 tauri::State<'_, EngineHandle> 注入（在 lib.rs 启动时 manage）。

use crate::engine::EngineHandle;
use crate::state::Status;
use chrono::Utc;
use serde::Serialize;

/// 前端可读的状态响应
#[derive(Debug, Clone, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub stage_index: usize,
    pub stage_type: String,
    pub remaining_seconds: u64,
    /// 形如 "44:32" 的剩余时间展示
    pub remaining_display: String,
}

fn fmt_display(secs: u64) -> String {
    let m = secs / 60;
    let s = secs % 60;
    format!("{:02}:{:02}", m, s)
}

fn status_str(s: Status) -> &'static str {
    match s {
        Status::Running => "running",
        Status::Paused => "paused",
        Status::Resting => "resting",
    }
}

/// 获取当前计时状态
#[tauri::command]
pub async fn get_status(engine: tauri::State<'_, EngineHandle>) -> Result<StatusResponse, String> {
    let (status, index, remaining) = engine.get_status().await;
    let stage_type = engine.current_stage_type().await;
    Ok(StatusResponse {
        status: status_str(status).to_string(),
        stage_index: index,
        stage_type: if stage_type == crate::config::StageType::Work {
            "work"
        } else {
            "rest"
        }
        .to_string(),
        remaining_seconds: remaining,
        remaining_display: fmt_display(remaining),
    })
}

/// 暂停计时（幂等，已暂停则无操作）
#[tauri::command]
pub async fn pause(engine: tauri::State<'_, EngineHandle>) -> Result<bool, String> {
    let now = Utc::now().timestamp();
    Ok(engine.pause(now).await)
}

/// 恢复计时
#[tauri::command]
pub async fn resume(engine: tauri::State<'_, EngineHandle>) -> Result<bool, String> {
    let now = Utc::now().timestamp();
    Ok(engine.resume(now).await)
}

/// 跳过当前休息阶段（仅 Resting 有效），进入下一个 work 阶段
#[tauri::command]
pub async fn skip_rest(engine: tauri::State<'_, EngineHandle>) -> Result<bool, String> {
    let now = Utc::now().timestamp();
    Ok(engine.skip_rest(now).await)
}

/// 重置计时：重读配置并从阶段 0 开始（用户托盘「重置计时」或配置变更后）
#[tauri::command]
pub async fn reset_timer(engine: tauri::State<'_, EngineHandle>) -> Result<(), String> {
    // 重新读取配置（可能已被前端修改保存）
    let (config, _fallback) = crate::config::Config::load();
    let now = Utc::now().timestamp();
    engine.reset_to_first(config, now).await;
    Ok(())
}

/// 保存配置（前端配置面板调用）。
/// 产品约定：保存只写文件，不自动生效；需用户「重置计时」或重启应用才生效。
/// 但快捷键是即时的（无需重置计时也会重新注册）。
#[tauri::command]
pub async fn save_config(
    engine: tauri::State<'_, EngineHandle>,
    app: tauri::AppHandle,
    config: crate::config::Config,
) -> Result<String, String> {
    config.validate().map_err(|e| e.to_string())?;
    // 读取旧快捷键用于注销
    let (old_config, _) = crate::config::Config::load();
    let old_shortcut = old_config.skip_shortcut.clone();
    let new_shortcut = config.skip_shortcut.clone();
    config.save().map_err(|e| e.to_string())?;
    // 仅更新引擎内存中的 config（不重置当前阶段），需用户主动重置才生效
    engine.update_config(config).await;
    // 快捷键即时更新
    if old_shortcut != new_shortcut {
        let engine_handle = engine.inner().clone();
        crate::shortcut::update(&app, &old_shortcut, &new_shortcut, engine_handle);
    }
    Ok("配置已保存，需重置计时或重启应用后生效（快捷键已即时更新）".to_string())
}

/// 读取当前配置（前端配置面板初始化用）
#[tauri::command]
pub async fn get_config() -> Result<crate::config::Config, String> {
    let (config, _fallback) = crate::config::Config::load();
    Ok(config)
}
