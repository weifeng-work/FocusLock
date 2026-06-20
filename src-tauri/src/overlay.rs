// overlay.rs — 休息遮罩窗口管理器（跨平台）
//
// 阶段 5（最高风险模块）：
// - fullscreen 模式：主显示器弹「巨大倒计时」遮罩，副显示器弹「纯黑半透明」遮罩
// - popup 模式：右下角小窗口
// - 热插拔：休息期间每 3s 重新枚举显示器，新增的补罩
// - 休息结束关闭所有遮罩
//
// 跨平台窗口属性（Win + Mac 通用）：
// - always_on_top: true
// - decorations: false（无边框）
// - transparent: true（配合前端 90% 黑色透明）
// - resizable: false
// - 位置和尺寸手动设为对应显示器矩形
// - Windows: 不用 fullscreen:true（会触发独占模式，导致多显示器切换问题）
// - macOS: 不用原生 fullscreen（会进独立 Space，无法覆盖副屏），用无边框置顶窗口
//
// 前端 URL：
// - 主显示器：/#/overlay?primary=1&remaining=<secs>  → Overlay.vue 大字倒计时 + 快捷键提示
// - 副显示器：/#/overlay?primary=0                     → Overlay.vue 纯黑半透明
// - popup：    /#/overlay?popup=1&remaining=<secs>     → Overlay.vue 紧凑倒计时
//
// 前端通过 listen("overlay-tick") 接收每秒倒计时更新。

use crate::platform::{create_capabilities, MonitorRect};
use std::collections::HashMap;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::Mutex;

/// 遮罩窗口 label 前缀。每显示器一个窗口：overlay-0, overlay-1, ...
const OVERLAY_PREFIX: &str = "overlay-";

/// popup 模式窗口 label
const POPUP_LABEL: &str = "overlay-popup";

/// 遮罩管理器
pub struct OverlayManager {
    /// 当前已打开的遮罩窗口 label → 显示器索引（popup 模式无显示器索引）
    windows: Mutex<HashMap<String, Option<usize>>>,
    /// 是否 fullscreen 模式（否则 popup）
    fullscreen: bool,
    /// 遮罩不透明度 0-100（传给前端）
    overlay_opacity: Mutex<u8>,
    /// 遮罩文案（传给前端）
    overlay_message: Mutex<String>,
}

impl OverlayManager {
    pub fn new(fullscreen: bool) -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            fullscreen,
            overlay_opacity: Mutex::new(95),
            overlay_message: Mutex::new("休息一下".to_string()),
        }
    }

    /// 设置当前遮罩参数（不透明度 0-100、文案）。由 lib.rs 在 RestStarted 时调用。
    pub async fn set_params(&self, opacity: u8, message: String) {
        *self.overlay_opacity.lock().await = opacity.min(100);
        *self.overlay_message.lock().await = message;
    }

    /// 展示休息遮罩。在引擎进入 Resting 时调用。
    pub async fn show(&self, app: &AppHandle<impl Runtime>, remaining: u64) {
        if self.fullscreen {
            self.show_fullscreen(app, remaining).await;
        } else {
            self.show_popup(app, remaining).await;
        }
    }

    /// 关闭所有遮罩。在休息结束/跳过时调用。
    pub async fn close_all(&self, app: &AppHandle<impl Runtime>) {
        let mut wins = self.windows.lock().await;
        let labels: Vec<String> = wins.keys().cloned().collect();
        for label in labels {
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.close();
            }
        }
        wins.clear();
    }

    /// 更新倒计时。每秒调用，前端 listen("overlay-tick") 接收。
    pub async fn tick(&self, app: &AppHandle<impl Runtime>, remaining: u64) {
        let _ = app.emit("overlay-tick", remaining);
    }

    async fn show_fullscreen(&self, app: &AppHandle<impl Runtime>, remaining: u64) {
        let monitors = create_capabilities()
            .monitor
            .list_monitors();
        let opacity = *self.overlay_opacity.lock().await;
        let message = self.overlay_message.lock().await.clone();
        let mut wins = self.windows.lock().await;
        for (idx, mon) in monitors.iter().enumerate() {
            let label = format!("{}{}", OVERLAY_PREFIX, idx);
            // 已存在则跳过
            if wins.contains_key(&label) {
                continue;
            }
            // 消息 URL encode（防止中文 / 特殊字符）
            let msg_enc = url_encode(&message);
            let url = format!(
                "/#/overlay?primary={}&remaining={}&opacity={}&msg={}",
                mon.is_primary, remaining, opacity, msg_enc
            );
            match self.create_overlay_window(app, &label, mon, &url, true).await {
                Ok(()) => {
                    wins.insert(label.clone(), Some(idx));
                    tracing::info!(
                        "遮罩窗口已创建：{} 显示器{} ({}x{})",
                        label,
                        idx,
                        mon.width,
                        mon.height
                    );
                }
                Err(e) => {
                    tracing::error!("创建遮罩窗口 {} 失败: {}", label, e);
                }
            }
        }
    }

    async fn show_popup(&self, app: &AppHandle<impl Runtime>, remaining: u64) {
        let mut wins = self.windows.lock().await;
        if wins.contains_key(POPUP_LABEL) {
            return;
        }
        let opacity = *self.overlay_opacity.lock().await;
        let message = self.overlay_message.lock().await.clone();
        // popup 用一个固定大小窗口，居中偏右下
        let rect = MonitorRect {
            x: -360,
            y: -260,
            width: 340,
            height: 220,
            is_primary: true,
        };
        let msg_enc = url_encode(&message);
        let url = format!("/#/overlay?popup=1&remaining={}&opacity={}&msg={}", remaining, opacity, msg_enc);
        match self.create_overlay_window(app, POPUP_LABEL, &rect, &url, false).await {
            Ok(()) => {
                wins.insert(POPUP_LABEL.to_string(), None);
            }
            Err(e) => {
                tracing::error!("创建 popup 遮罩失败: {}", e);
            }
        }
    }

    /// 创建单个遮罩窗口
    async fn create_overlay_window(
        &self,
        app: &AppHandle<impl Runtime>,
        label: &str,
        mon: &MonitorRect,
        url: &str,
        fullscreen_style: bool,
    ) -> tauri::Result<()> {
        let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
            .title("FocusLock 休息中")
            .decorations(false)
            .always_on_top(true)
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            .skip_taskbar(true)
            .shadow(false);

        if fullscreen_style {
            // 全屏遮罩：覆盖整个显示器矩形
            builder = builder
                .position(mon.x as f64, mon.y as f64)
                .inner_size(mon.width as f64, mon.height as f64)
                .transparent(true);
        } else {
            // popup：固定尺寸，定位到右下角（主显示器）
            let primary = create_capabilities().monitor.list_monitors()
                .into_iter()
                .find(|m| m.is_primary);
            if let Some(p) = primary {
                let x = p.x + p.width - mon.width - 40;
                let y = p.y + p.height - mon.height - 60;
                builder = builder.position(x as f64, y as f64);
            }
            builder = builder
                .inner_size(mon.width as f64, mon.height as f64)
                .transparent(false);
        }

        let win = builder.build()?;
        // 防失焦置顶：监听失焦事件重新置顶（粗略实现，后续可加定时 re-raise）
        let win_clone = win.clone();
        win.on_window_event(move |event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let _ = win_clone.set_always_on_top(true);
            }
        });
        Ok(())
    }
}

/// 启动热插拔检测 task：休息期间每 3s 重新枚举显示器，新增的补罩。
/// 由 lib.rs 在每次 RestStarted 时启动，RestEnded/Skipped 时停止。
pub fn spawn_hotplug_watcher(
    app: AppHandle<impl Runtime>,
    manager: std::sync::Arc<OverlayManager>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3));
        interval.tick().await; // 跳过首次
        loop {
            interval.tick().await;
            // 仅在仍有遮罩打开时检查（休息中）
            let wins = manager.windows.lock().await;
            let active = !wins.is_empty();
            drop(wins);
            if !active {
                break;
            }
            // 检查显示器数量变化，补罩
            let monitors = create_capabilities().monitor.list_monitors();
            let mut wins = manager.windows.lock().await;
            for (idx, mon) in monitors.iter().enumerate() {
                let label = format!("{}{}", OVERLAY_PREFIX, idx);
                if !wins.contains_key(&label) {
                    // 新增显示器，补罩
                    drop(wins);
                    let url = format!(
                        "/#/overlay?primary={}&remaining=0",
                        mon.is_primary
                    );
                    if let Ok(()) = manager
                        .create_overlay_window(&app, &label, mon, &url, true)
                        .await
                    {
                        let mut wins2 = manager.windows.lock().await;
                        wins2.insert(label, Some(idx));
                    }
                    wins = manager.windows.lock().await;
                }
            }
        }
    })
}

/// URL-encode 用于 query string 里的文案（覆盖 ASCII 非保留字符 + 非 ASCII）
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}
