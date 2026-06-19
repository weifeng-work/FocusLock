// FocusLock 库入口
// 所有跨平台业务逻辑在此组织，main.rs 仅调用 run()

mod commands;
mod config;
mod engine;
mod notify;
mod overlay;
mod platform;
mod shortcut;
mod state;
mod tray;
mod tray_icons;

use chrono::Utc;
use std::sync::Arc;
use tauri::{Emitter, Manager};

use config::{Config, RestReminderMode, StageType};
use engine::{check_and_reset, spawn_engine, EngineEvent, ResetVerdict};
use state::{AppState, Status};

/// 计算指定阶段的总秒数
fn stage_total_seconds(config: &Config, index: usize) -> u64 {
    u64::from(config.stages[index].minutes) * 60
}

/// 启动时的状态初始化逻辑：
/// 1. 读 config（非法回退默认）
/// 2. 读 state：
///    - 无 state → 全新启动，阶段 0
///    - 有 state + 过夜重置判定 Yes → 重置阶段 0
///    - 有 state + 判定 No → 恢复；若 Paused → 自动转 Running（产品约定）
/// 返回 (state, stage_started_at)
fn bootstrap_state(config: &Config) -> (AppState, i64) {
    let now = Utc::now().timestamp();
    let threshold_secs = u64::from(config.reset_threshold_minutes) * 60;

    match AppState::load() {
        None => {
            tracing::info!("无 state，全新启动，进入阶段 0。");
            let total = stage_total_seconds(config, 0);
            (AppState::fresh_first_stage(total, now), now)
        }
        Some(s) => {
            match check_and_reset(&s, now, threshold_secs) {
                ResetVerdict::Yes => {
                    tracing::info!("检测到长时间离开（delta={}s），重置到阶段 0。", now - s.last_active_timestamp);
                    let total = stage_total_seconds(config, 0);
                    (AppState::fresh_first_stage(total, now), now)
                }
                ResetVerdict::No => {
                    // 恢复
                    let mut s = s;
                    let total = stage_total_seconds(config, s.current_stage_index);
                    if s.status == Status::Paused {
                        // 产品约定：暂停状态重启后自动恢复 Running（从冻结 remaining 继续）
                        tracing::info!("上次处于暂停，自动恢复计时。");
                        s.status =
                            if config.stages[s.current_stage_index].stage_type == StageType::Work {
                                Status::Running
                            } else {
                                Status::Resting
                            };
                    }
                    let elapsed = total.saturating_sub(s.remaining_seconds);
                    let started = now - elapsed as i64;
                    tracing::info!(
                        "恢复状态：阶段 {}，{:?}，剩余 {}s",
                        s.current_stage_index,
                        s.status,
                        s.remaining_seconds
                    );
                    (s, started)
                }
            }
        }
    }
}

/// 应用启动入口
pub fn run() {
    // 初始化日志
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "focuslock=info,warn".into()),
        )
        .try_init();

    tracing::info!("FocusLock 启动中…");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tracing::info!("检测到重复启动，已忽略。");
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // 1. 加载配置 + 初始化状态
            let (config, fallback) = Config::load();
            if fallback {
                tracing::warn!("配置非法已回退默认，建议用户检查 config.json");
            }
            let skip_accelerator = config.skip_shortcut.clone();
            // 提前读取遮罩模式（从当前方案获取，config 会被 move 给 spawn_engine）
            let fullscreen = config.schemes.first()
                .map(|s| matches!(s.rest_reminder_mode, RestReminderMode::Fullscreen))
                .unwrap_or(true);
            let (state, started_at) = bootstrap_state(&config);

            // 2. 启动计时引擎
            //    spawn_engine 返回 tick_loop future，由我们用 tauri::async_runtime::spawn 启动
            //    （setup 闭包不在 Tokio runtime context，不能直接 tokio::spawn）
            let (handle, mut event_rx, tick_future) = spawn_engine(config, state, started_at);
            tauri::async_runtime::spawn(tick_future);

            // 2.1 注册全局快捷键（跳过休息）
            shortcut::register(app.handle(), &skip_accelerator, handle.clone());

            // 3. 引擎事件桥接到 Tauri event（供前端订阅）
            //    前端 listen("engine-event") 即可收到状态变更
            //    同时按事件类型分发系统通知（阶段 4）+ 遮罩管理（阶段 5）
            let app_handle = app.handle().clone();
            // 遮罩管理器：fullscreen/popup 模式
            let overlay_mgr = Arc::new(overlay::OverlayManager::new(fullscreen));
            let overlay_mgr_for_task = overlay_mgr.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(ev) = event_rx.recv().await {
                    tracing::info!("引擎事件: {:?}", ev);
                    let _ = app_handle.emit("engine-event", &ev);
                    notify::on_engine_event(&app_handle, &ev);
                    // 遮罩生命周期管理
                    match &ev {
                        EngineEvent::RestStarted { remaining, .. } => {
                            overlay_mgr_for_task.show(&app_handle, *remaining).await;
                        }
                        EngineEvent::RestEnded | EngineEvent::RestSkipped => {
                            overlay_mgr_for_task.close_all(&app_handle).await;
                        }
                        EngineEvent::StatusChanged { status, .. } => {
                            if *status == Status::Paused {
                                overlay_mgr_for_task.close_all(&app_handle).await;
                            }
                        }
                        _ => {}
                    }
                }
            });
            // 每秒倒计时推送给遮罩前端
            let app_handle2 = app.handle().clone();
            let overlay_mgr_tick = overlay_mgr.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
                interval.tick().await;
                loop {
                    interval.tick().await;
                    let engine = match app_handle2.try_state::<engine::EngineHandle>() {
                        Some(e) => e.inner().clone(),
                        None => continue,
                    };
                    let (status, _, remaining) = engine.get_status().await;
                    if status == Status::Resting {
                        overlay_mgr_tick.tick(&app_handle2, remaining).await;
                    }
                }
            });

            // 4. 托盘：创建 + 启动 tooltip/图标刷新 task
            tray::create_tray(app.handle())?;
            tray::spawn_tray_updater(app.handle().clone(), handle.clone());

            // 4.1 拦截主窗口（配置面板）关闭请求：改为隐藏，不退出应用。
            //     托盘应用常驻后台，关闭配置窗口不应结束进程。
            //     用户通过托盘菜单「退出」才会真正退出（app.exit(0)）。
            if let Some(main_win) = app.get_webview_window("main") {
                let win_clone = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                        tracing::info!("配置窗口关闭请求已拦截，改为隐藏（应用继续后台运行）");
                    }
                });
            }

            // 4.2 macOS 首启触发通知授权
            notify::trigger_permission_prompt(app.handle());

            // 5. 引擎句柄 + 遮罩管理器注入 Tauri managed state（供 command 层访问）
            app.manage(handle);
            app.manage(overlay_mgr);

            tracing::info!("FocusLock 已就绪。");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::pause,
            commands::resume,
            commands::skip_rest,
            commands::reset_timer,
            commands::save_config,
            commands::get_config,
            commands::check_update,
            commands::copy_custom_sound,
            commands::get_sound_files,
            commands::delete_sound_file,
            commands::get_app_data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("FocusLock 启动失败");
}
