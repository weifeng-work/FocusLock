// commands.rs — 暴露给前端的 Tauri command
//
// 阶段 3 起接入：前端（配置面板 / 遮罩页）通过 invoke 调用这些命令。
// 引擎句柄通过 tauri::State<'_, EngineHandle> 注入（在 lib.rs 启动时 manage）。

use crate::engine::EngineHandle;
use crate::state::Status;
use chrono::Utc;
use serde::Serialize;

/// 检查更新响应
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResponse {
    /// 当前安装版本
    pub current: String,
    /// GitHub 最新版本 tag
    pub latest: String,
    /// 最新版 Release 页面 URL
    pub url: String,
    /// 是否有新版本可用
    pub has_update: bool,
}

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

/// 检查更新：查询 GitHub releases API 获取最新版本信息
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<UpdateCheckResponse, String> {
    // 从 tauri.conf.json 读取当前版本
    let current = app.config().version.clone().unwrap_or_else(|| "0.0.0".into());

    // 调用 GitHub Releases API
    let client = reqwest::Client::builder()
        .user_agent("FocusLock")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .get("https://api.github.com/repos/weifeng-work/FocusLock/releases/latest")
        .send()
        .await
        .map_err(|e| format!("请求 GitHub API 失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API 返回错误: HTTP {}", resp.status()));
    }

    #[derive(Deserialize)]
    struct GhRelease {
        tag_name: String,
        html_url: String,
    }

    use serde::Deserialize;

    let release: GhRelease = resp
        .json()
        .await
        .map_err(|e| format!("解析 GitHub 响应失败: {}", e))?;

    // 去掉 v 前缀后比较
    let latest_clean = release.tag_name.trim_start_matches('v').to_string();
    let current_clean = current.trim_start_matches('v').to_string();

    // 简单版本号字符串比较（语义化版本）
    let has_update = latest_clean != current_clean;

    Ok(UpdateCheckResponse {
        current: current_clean,
        latest: latest_clean,
        url: release.html_url,
        has_update,
    })
}

/// 音效文件信息（前端用）
#[derive(Debug, Clone, Serialize)]
pub struct SoundFileInfo {
    pub name: String,
    pub file: String,
    pub path: String,
}

/// 复制自定义音效文件到应用数据目录
#[tauri::command]
pub async fn copy_custom_sound(source_path: String) -> Result<SoundFileInfo, String> {
    use std::path::Path;
    use tokio::fs;

    let source = Path::new(&source_path);
    if !source.exists() {
        return Err("源文件不存在".to_string());
    }

    // 检查文件扩展名
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["mp3", "wav", "aac", "ogg", "flac", "m4a"].contains(&ext.as_str()) {
        return Err("不支持的音频格式，请选择 mp3/wav/aac/ogg/flac/m4a 文件".to_string());
    }

    // 目标目录：%APPDATA%/FocusLock/sounds/
    let sounds_dir = crate::config::Config::data_dir().join("sounds");
    fs::create_dir_all(&sounds_dir)
        .await
        .map_err(|e| format!("创建音效目录失败: {}", e))?;

    let file_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("无效的文件名")?;
    let dest = sounds_dir.join(file_name);

    // 复制文件
    fs::copy(source, &dest)
        .await
        .map_err(|e| format!("复制文件失败: {}", e))?;

    let display_name = source
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or(file_name)
        .to_string();

    Ok(SoundFileInfo {
        name: display_name,
        file: file_name.to_string(),
        path: dest.to_string_lossy().to_string(),
    })
}

/// 获取已保存的自定义音效列表
#[tauri::command]
pub async fn get_sound_files() -> Result<Vec<SoundFileInfo>, String> {
    use tokio::fs;

    let sounds_dir = crate::config::Config::data_dir().join("sounds");

    if !sounds_dir.exists() {
        return Ok(vec![]);
    }

    let mut files = vec![];
    let mut entries = fs::read_dir(&sounds_dir)
        .await
        .map_err(|e| format!("读取音效目录失败: {}", e))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| format!("读取目录项失败: {}", e))?
    {
        let path = entry.path();
        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ["mp3", "wav", "aac", "ogg", "flac", "m4a"].contains(&ext.as_str()) {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                let display_name = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&file_name)
                    .to_string();
                files.push(SoundFileInfo {
                    name: display_name,
                    file: file_name,
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }

    Ok(files)
}

/// 删除自定义音效文件
#[tauri::command]
pub async fn delete_sound_file(file_name: String) -> Result<bool, String> {
    use tokio::fs;

    let sounds_dir = crate::config::Config::data_dir().join("sounds");
    let file_path = sounds_dir.join(&file_name);

    if !file_path.exists() {
        return Err("文件不存在".to_string());
    }

    fs::remove_file(&file_path)
        .await
        .map_err(|e| format!("删除文件失败: {}", e))?;

    Ok(true)
}

/// 获取应用数据目录路径（前端用它构建音频文件 URL）
#[tauri::command]
pub async fn get_app_data_dir() -> Result<String, String> {
    let dir = crate::config::Config::data_dir();
    Ok(dir.to_string_lossy().to_string())
}

/// 读取音效文件并返回 base64 编码
#[tauri::command]
pub async fn read_sound_file(file_name: String) -> Result<String, String> {
    use std::path::Path;
    use tokio::fs;

    let sounds_dir = crate::config::Config::data_dir().join("sounds");
    let file_path = sounds_dir.join(&file_name);

    if !file_path.exists() {
        return Err("文件不存在".to_string());
    }

    // 读取文件
    let data = fs::read(&file_path)
        .await
        .map_err(|e| format!("读取文件失败: {}", e))?;

    // 转换为 base64
    let base64 = base64::engine::general_purpose::STANDARD.encode(&data);
    Ok(base64)
}

/// 用系统默认浏览器打开外部 URL
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    webbrowser::open(&url)
        .map_err(|e| format!("无法打开链接: {}", e))
}
