// engine/timer.rs — 计时引擎核心
//
// 设计要点（对应实施计划阶段 2）：
// 1. 后台 tokio task 每秒 tick，但**不依赖 sleep 累减**计时；
//    每次推进用 now - stage_started_at 重新计算 remaining，避免系统挂起失准。
// 2. 状态机：Running ⇄ Resting；Pausing 可从两者进入，恢复时回到原状态。
// 3. 暂停冻结 remaining_seconds；恢复时重设 stage_started_at = now - (total - remaining)。
// 4. 暂停期间不触发过夜重置（reset.rs 的 check 直接 return）。
// 5. 每 60 秒 + 每次状态切换写 state.json。
// 6. 工作结束前 N 分钟（config.notify_before_work_end_minutes）发一次「准备休息」事件。
// 7. 阶段归零时推进到下一阶段并切换状态，发对应事件给前端。
//
// 引擎通过 EngineHandle 暴露操作（pause/resume/skip/reset/get_status），
// 由 Tauri command 层调用。事件通过 emit 回调发出，由上层桥接到 Tauri event 或通知。

use crate::config::{
    Config, OverlayStyle, PeriodEndAction, RestReminderMode, Scheme,
    SoundType, Stage, StageType,
};
use crate::engine::stage::StageCursor;
use crate::state::{AppState, Status};
use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;

/// 引擎对外发出的事件，上层（commands.rs / lib.rs）负责桥接到 Tauri event / 系统通知
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    /// 工作阶段开始（剩余秒数）
    WorkStarted { remaining: u64 },
    /// 工作即将结束，准备休息（剩余秒数，应 <= notify_before * 60）
    PrepareRest { remaining: u64 },
    /// 进入休息阶段（剩余秒数，提醒模式）
    RestStarted {
        remaining: u64,
        mode: RestReminderMode,
    },
    /// 休息结束，进入下一阶段
    RestEnded,
    /// 已跳过休息，进入下一 work 阶段
    RestSkipped,
    /// 状态变更（暂停/恢复/阶段切换），前端可据此刷新
    StatusChanged { status: Status, remaining: u64 },
    /// 检测到长时间离开，已重置
    ResetDueToInactivity,
    /// 播放音效（音效类型：work_end / rest_end）
    PlaySound { sound_type: String },
    /// 时间段结束，执行时段结束动作（弹窗/全屏/黑屏/无操作）
    PeriodEndedAction { action: PeriodEndAction },
}

/// 引擎内部状态
struct EngineInner {
    config: Config,
    state: AppState,
    /// 当前使用的方案 stages 缓存（随 scheme_id 变更而刷新）
    cached_stages: Vec<Stage>,
    /// 当前生效的作息表 ID（来自 weekly 周配置）
    current_routine_id: Option<String>,
    /// 当前所处时间段的索引（若不在任何时间段内则为 None）
    current_period_idx: Option<usize>,
    /// 当前阶段开始的绝对时间戳（秒）。暂停时不再推进。
    stage_started_at: i64,
    /// 本阶段是否已发过「准备休息」通知（避免重复发）
    prepare_rest_sent: bool,
    /// 上次写 state.json 的秒时间戳（节流：每 60s 一次）
    last_persist_at: i64,
}

/// 引擎句柄，Tauri command 层持有其克隆来操作引擎
#[derive(Clone)]
pub struct EngineHandle {
    inner: Arc<Mutex<EngineInner>>,
    /// 事件发送端，后台 task 持有接收端
    event_tx: tokio::sync::mpsc::UnboundedSender<EngineEvent>,
}

impl EngineHandle {
    /// 获取当前状态快照
    pub async fn get_status(&self) -> (Status, usize, u64) {
        let g = self.inner.lock().await;
        (g.state.status, g.state.current_stage_index, g.state.remaining_seconds)
    }

    /// 获取当前阶段类型（供托盘/命令层组装完整状态）
    pub async fn current_stage_type(&self) -> StageType {
        let g = self.inner.lock().await;
        g.current_stage_type()
    }

    /// 暂停。仅 Running/Resting 可暂停。
    /// 暂停时冻结 remaining（由 tick 在恢复时重设 started_at）。
    pub async fn pause(&self, now: i64) -> bool {
        let mut g = self.inner.lock().await;
        let st = g.state.status;
        if st == Status::Paused {
            return false; // 已暂停，幂等
        }
        // 先把当前 remaining 落到 state（基于时间戳计算）
        g.recompute_remaining(now);
        g.state.status = Status::Paused;
        g.state.last_active_timestamp = now;
        let remaining = g.state.remaining_seconds;
        let _ = g.persist();
        drop(g);
        let _ = self.event_tx.send(EngineEvent::StatusChanged {
            status: Status::Paused,
            remaining,
        });
        true
    }

    /// 恢复。从 Paused 回到 Running（无论暂停前是 Running 还是 Resting，统一回 Running 风格；
    /// 注意：产品约定暂停重启后自动恢复 Running，本方法是「运行中用户点恢复」，
    /// 与「重启时自动恢复」逻辑在 lib.rs 启动流程里处理）。
    pub async fn resume(&self, now: i64) -> bool {
        let mut g = self.inner.lock().await;
        if g.state.status != Status::Paused {
            return false;
        }
        // 恢复时重设 stage_started_at，使 remaining 从冻结值继续推进
        g.refresh_cached_stages();
        let total = StageCursor::new(&g.cached_stages, g.state.current_stage_index)
            .current_total_seconds();
        let elapsed = total.saturating_sub(g.state.remaining_seconds);
        g.stage_started_at = now - elapsed as i64;
        // 恢复到的状态：若当前阶段是 work → Running；rest → Resting
        g.state.status = if g.current_stage_type() == StageType::Work {
            Status::Running
        } else {
            Status::Resting
        };
        g.state.last_active_timestamp = now;
        g.prepare_rest_sent = false; // 恢复后允许重新发准备通知（若仍在窗口内）
        let status = g.state.status;
        let remaining = g.state.remaining_seconds;
        let _ = g.persist();
        drop(g);
        let _ = self.event_tx.send(EngineEvent::StatusChanged { status, remaining });
        true
    }

    /// 跳过当前休息阶段，进入下一个 work 阶段。
    /// 仅 Resting 状态有效。
    pub async fn skip_rest(&self, now: i64) -> bool {
        let mut g = self.inner.lock().await;
        if g.state.status != Status::Resting {
            return false;
        }
        let cursor = StageCursor::new(&g.cached_stages, g.state.current_stage_index);
        let (new_index, _skipped) = cursor.skip_rest_to_next_work();
        g.enter_stage(new_index, now);
        let remaining = g.state.remaining_seconds;
        let _ = g.persist();
        drop(g);
        let _ = self.event_tx.send(EngineEvent::RestSkipped);
        let _ = self.event_tx.send(EngineEvent::WorkStarted { remaining });
        let _ = self.event_tx.send(EngineEvent::StatusChanged {
            status: Status::Running,
            remaining,
        });
        true
    }

    /// 重置到第一阶段（用户托盘「重置计时」或配置变更后触发）。
    /// 重新读 config（由调用方传入新 config）。
    pub async fn reset_to_first(&self, new_config: Config, now: i64) {
        let mut g = self.inner.lock().await;
        g.config = new_config;
        g.enter_stage(0, now);
        let remaining = g.state.remaining_seconds;
        let _ = g.persist();
        drop(g);
        let _ = self.event_tx.send(EngineEvent::WorkStarted { remaining });
        let _ = self.event_tx.send(EngineEvent::StatusChanged {
            status: Status::Running,
            remaining,
        });
    }

    /// 更新配置但不重置（保存配置后调用；产品约定需手动重置或重启才生效，
    /// 因此这里只存新 config，引擎继续按旧节奏跑，直到 reset_to_first 被调用）。
    pub async fn update_config(&self, new_config: Config) {
        let mut g = self.inner.lock().await;
        g.config = new_config;
    }
}

impl EngineInner {
    /// 根据 state.scheme_id 刷新 cached_stages
    fn refresh_cached_stages(&mut self) {
        self.cached_stages = self
            .config
            .schemes
            .iter()
            .find(|s| s.id == self.state.scheme_id)
            .map(|s| s.stages.clone())
            .unwrap_or_else(|| {
                // 找不到方案时 fallback 到第一个方案
                tracing::warn!("方案 {} 未找到，fallback 到第一个方案", self.state.scheme_id);
                self.config.schemes.first().cloned().map(|s| s.stages).unwrap_or_default()
            });
    }

    fn current_stage_type(&self) -> StageType {
        self.cached_stages[self.state.current_stage_index].stage_type
    }

    /// 获取当前方案（根据 state.scheme_id）
    fn current_scheme(&self) -> Option<&Scheme> {
        self.config.schemes.iter().find(|s| s.id == self.state.scheme_id)
    }

    /// 基于系统时间戳重新计算 remaining_seconds
    fn recompute_remaining(&mut self, now: i64) {
        if self.state.status == Status::Paused {
            return; // 暂停时不动
        }
        let total = StageCursor::new(&self.cached_stages, self.state.current_stage_index)
            .current_total_seconds();
        let elapsed = (now - self.stage_started_at).max(0) as u64;
        self.state.remaining_seconds = total.saturating_sub(elapsed);
    }

    /// 进入指定阶段：设置 index、started_at、remaining、status、清 prepare_rest_sent
    /// 若 scheme_id 与 state 中不同，先刷新 cached_stages
    fn enter_stage(&mut self, index: usize, now: i64) {
        // 检查当前方案名是否与 state 中的一致（供外部在切换方案后调用）
        self.refresh_cached_stages();

        self.state.current_stage_index = index;
        self.stage_started_at = now;
        let total = StageCursor::new(&self.cached_stages, index).current_total_seconds();
        self.state.remaining_seconds = total;
        self.state.status = if self.cached_stages[index].stage_type == StageType::Work {
            Status::Running
        } else {
            Status::Resting
        };
        self.state.last_active_timestamp = now;
        self.prepare_rest_sent = false;
    }

    /// 持久化 state.json
    fn persist(&self) -> std::io::Result<()> {
        self.state.save()
    }

    /// 检查作息表调度，需要时切换方案 / 触发时段结束动作。
    /// 由 tick_loop 每分钟调用一次。
    pub fn check_schedule(&mut self, now: i64, tx: &tokio::sync::mpsc::UnboundedSender<EngineEvent>) {
        let now_local = Local::now();
        let routine_id = self.config.weekly.get_routine_id(now_local.weekday()).to_string();
        let target = self.config.resolve_current_schedule(now_local);

        match target {
            Some((ref new_scheme_id, period_idx)) => {
                let period_changed = self.current_period_idx != Some(period_idx)
                    || self.current_routine_id.as_deref() != Some(routine_id.as_str());

                // 刚离开上一个时段 → 发送上一个时段的结束动作
                if period_changed && self.current_period_idx.is_some() {
                    Self::send_period_end_action(
                        &self.config,
                        self.current_routine_id.clone(),
                        self.current_period_idx,
                        tx,
                    );
                }

                // 方案变了 → 切换到新方案并从第一阶段开始
                if *new_scheme_id != self.state.scheme_id {
                    self.state.scheme_id = new_scheme_id.clone();
                    self.refresh_cached_stages();
                    self.enter_stage(0, now);
                    let _ = self.persist();
                    let _ = tx.send(EngineEvent::WorkStarted {
                        remaining: self.state.remaining_seconds,
                    });
                    let _ = tx.send(EngineEvent::StatusChanged {
                        status: self.state.status,
                        remaining: self.state.remaining_seconds,
                    });
                }

                self.current_routine_id = Some(routine_id);
                self.current_period_idx = Some(period_idx);
            }
            None => {
                // 不在任何时间段内（跨时段间隙，如 12:00-14:00）
                if self.current_period_idx.is_some() {
                    Self::send_period_end_action(
                        &self.config,
                        self.current_routine_id.clone(),
                        self.current_period_idx,
                        tx,
                    );
                    self.current_period_idx = None;
                    self.current_routine_id = None;
                }
            }
        }
    }

    /// 发送上一个时段的结束动作事件（内部辅助方法）
    fn send_period_end_action(
        config: &Config,
        routine_id: Option<String>,
        period_idx: Option<usize>,
        tx: &tokio::sync::mpsc::UnboundedSender<EngineEvent>,
    ) {
        if let Some(ref rid) = routine_id {
            if let Some(idx) = period_idx {
                if let Some(routine) = config.routines.iter().find(|r| r.id == *rid) {
                    if let Some(period) = routine.periods.get(idx) {
                        let _ = tx.send(EngineEvent::PeriodEndedAction {
                            action: period.end_action.clone(),
                        });
                    }
                }
            }
        }
    }
}

/// 后台 tick 循环 future。由 spawn_engine 返回，调用方负责 spawn（用 tauri::async_runtime::spawn 或 tokio::spawn）。
///
/// 这样设计是因为 Tauri setup 闭包不在 Tokio runtime context，不能直接 tokio::spawn。
/// 调用方根据环境选择合适的 spawn 方式。
async fn tick_loop(
    inner: Arc<Mutex<EngineInner>>,
    tx: tokio::sync::mpsc::UnboundedSender<EngineEvent>,
) {
    let mut tick = interval(Duration::from_secs(1));
    // 第一个 tick 立即触发，但我们跳过首次（启动时状态已就绪）
    tick.tick().await;
    loop {
        tick.tick().await;
        let now = Utc::now().timestamp();
        let mut g = inner.lock().await;

        // 暂停状态：不推进、不重置，仅更新 last_active（用于恢复时判定）
        if g.state.status == Status::Paused {
            g.state.last_active_timestamp = now;
            continue;
        }

        // 过夜重置判定（暂停态已在上面 continue，这里都是活跃态）
        let threshold_secs = u64::from(g.config.reset_threshold_minutes) * 60;
        let delta = (now - g.state.last_active_timestamp).max(0) as u64;
        if delta > threshold_secs {
            g.enter_stage(0, now);
            let _ = g.persist();
            let _ = tx.send(EngineEvent::ResetDueToInactivity);
            let _ = tx.send(EngineEvent::WorkStarted {
                remaining: g.state.remaining_seconds,
            });
            continue;
        }

        // 检查作息表调度（每分钟自动切换方案 / 触发时段结束动作）
        g.check_schedule(now, &tx);

        // 正常推进
        g.recompute_remaining(now);

        match g.state.status {
            Status::Running => {
                let notify_secs = u64::from(g.config.notify_before_work_end_minutes) * 60;
                if !g.prepare_rest_sent
                    && g.state.remaining_seconds <= notify_secs
                    && g.state.remaining_seconds > 0
                {
                    g.prepare_rest_sent = true;
                    let _ = tx.send(EngineEvent::PrepareRest {
                        remaining: g.state.remaining_seconds,
                    });
                }
        if g.state.remaining_seconds == 0 {
                    let next = StageCursor::new(&g.cached_stages, g.state.current_stage_index)
                        .advance();
                    g.enter_stage(next, now);
                    let _ = g.persist();
                    if g.current_stage_type() == StageType::Work {
                        let _ = tx.send(EngineEvent::WorkStarted {
                            remaining: g.state.remaining_seconds,
                        });
                    } else {
                        // 发送工作结束提示音事件（从当前方案获取）
                        if let Some(scheme) = g.current_scheme() {
                            let sound = &scheme.work_end_sound;
                            let sound_str = match sound {
                                SoundType::None => "none".to_string(),
                                SoundType::Builtin => "builtin".to_string(),
                                SoundType::Custom(f) => format!("custom:{}", f),
                            };
                            if sound_str != "none" {
                                let _ = tx.send(EngineEvent::PlaySound {
                                    sound_type: format!("work_end:{}", sound_str),
                                });
                            }
                        }
                        let mode = g.current_scheme()
                            .map(|s| s.rest_reminder_mode)
                            .unwrap_or(crate::config::RestReminderMode::Fullscreen);
                        let _ = tx.send(EngineEvent::RestStarted {
                            remaining: g.state.remaining_seconds,
                            mode,
                        });
                    }
                    let _ = tx.send(EngineEvent::StatusChanged {
                        status: g.state.status,
                        remaining: g.state.remaining_seconds,
                    });
                }
            }
            Status::Resting => {
            if g.state.remaining_seconds == 0 {
                    let next = StageCursor::new(&g.cached_stages, g.state.current_stage_index)
                        .advance();
                    // 发送休息结束提示音事件
                    let sound = if let Some(scheme) = g.current_scheme() {
                        &scheme.rest_end_sound
                    } else {
                        &crate::config::SoundType::Builtin
                    };
                    let sound_str = match sound {
                        crate::config::SoundType::None => "none".to_string(),
                        crate::config::SoundType::Builtin => "builtin".to_string(),
                        crate::config::SoundType::Custom(f) => format!("custom:{}", f),
                    };
                    if sound_str != "none" {
                        let _ = tx.send(EngineEvent::PlaySound {
                            sound_type: format!("rest_end:{}", sound_str),
                        });
                    }
                    let _ = tx.send(EngineEvent::RestEnded);
                    g.enter_stage(next, now);
                    let _ = g.persist();
                    if g.current_stage_type() == StageType::Work {
                        let _ = tx.send(EngineEvent::WorkStarted {
                            remaining: g.state.remaining_seconds,
                        });
                    } else {
                        let mode = g.current_scheme()
                            .map(|s| s.rest_reminder_mode)
                            .unwrap_or(crate::config::RestReminderMode::Fullscreen);
                        let _ = tx.send(EngineEvent::RestStarted {
                            remaining: g.state.remaining_seconds,
                            mode,
                        });
                    }
                    let _ = tx.send(EngineEvent::StatusChanged {
                        status: g.state.status,
                        remaining: g.state.remaining_seconds,
                    });
                }
            }
            Status::Paused => unreachable!("paused handled above"),
        }

        // 更新 last_active + 节流持久化（每 60s）
        g.state.last_active_timestamp = now;
        if (now - g.last_persist_at).max(0) as u64 >= 60 {
            g.last_persist_at = now;
            let _ = g.persist();
        }
    }
}

/// 创建引擎，返回 (handle, 事件接收端, tick_loop future)。
/// 初始 config / state 由调用方决定（首次启动 vs 恢复）。
///
/// 调用方负责 spawn tick_future（Tauri 用 `tauri::async_runtime::spawn`，测试用 `tokio::spawn`）。
/// 不在内部 spawn 是因为 Tauri setup 闭包不在 Tokio runtime context。
pub fn spawn_engine(
    config: Config,
    state: AppState,
    started_at: i64,
) -> (
    EngineHandle,
    tokio::sync::mpsc::UnboundedReceiver<EngineEvent>,
    std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let inner = Arc::new(Mutex::new(EngineInner {
        config: config.clone(),
        state: state.clone(),
        cached_stages: config.stages.clone(),
        current_routine_id: None,
        current_period_idx: None,
        stage_started_at: started_at,
        prepare_rest_sent: false,
        last_persist_at: started_at,
    }));
    let handle = EngineHandle {
        inner: inner.clone(),
        event_tx: tx.clone(),
    };

    let future = Box::pin(tick_loop(inner, tx));
    (handle, rx, future)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Stage, StageType};

    fn cfg() -> Config {
        Config {
            stages: vec![
                Stage {
                    stage_type: StageType::Work,
                    minutes: 1,
                },
                Stage {
                    stage_type: StageType::Rest,
                    minutes: 1,
                },
            ],
            reset_threshold_minutes: 30,
            notify_before_work_end_minutes: 1,
            ..Config::default()
        }
    }

    #[test]
    fn recompute_remaining_uses_timestamp() {
        // 验证「基于时间戳推进」而非 sleep 减一
        let config = cfg();
        let state = AppState::fresh_first_stage(60, 1000);
        let mut inner = EngineInner {
            config: config.clone(),
            state: state.clone(),
            stage_started_at: 1000,
            prepare_rest_sent: false,
            last_persist_at: 1000,
        };
        // 30 秒后，剩余 30
        inner.recompute_remaining(1030);
        assert_eq!(inner.state.remaining_seconds, 30);
        // 60 秒后，归零
        inner.recompute_remaining(1060);
        assert_eq!(inner.state.remaining_seconds, 0);
        // 90 秒后，仍为 0（saturating）
        inner.recompute_remaining(1090);
        assert_eq!(inner.state.remaining_seconds, 0);
    }

    #[test]
    fn pause_freezes_remaining_resume_continues() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = cfg();
            let state = AppState::fresh_first_stage(60, 1000);
            let (h, _rx, tick_fut) = spawn_engine(config, state, 1000);
            let _tick_task = tokio::spawn(tick_fut);
            // 暂停（伪时间 1020，剩余应 40）
            assert!(h.pause(1020).await);
            let (st, _, rem) = h.get_status().await;
            assert_eq!(st, Status::Paused);
            assert_eq!(rem, 40);
            // 恢复（伪时间 2000，远离 1020，但暂停期不应推进）
            assert!(h.resume(2000).await);
            let (st, _, rem) = h.get_status().await;
            assert_eq!(st, Status::Running);
            assert_eq!(rem, 40); // 仍是 40，暂停期流逝不计
        });
    }

    #[test]
    fn skip_rest_goes_to_next_work() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut config = cfg();
            // [work, rest, rest, work] 当前在 rest(1)
            config.stages = vec![
                Stage {
                    stage_type: StageType::Work,
                    minutes: 1,
                },
                Stage {
                    stage_type: StageType::Rest,
                    minutes: 1,
                },
                Stage {
                    stage_type: StageType::Rest,
                    minutes: 1,
                },
                Stage {
                    stage_type: StageType::Work,
                    minutes: 1,
                },
            ];
            let mut state = AppState::fresh_first_stage(60, 1000);
            state.current_stage_index = 1;
            state.status = Status::Resting;
            state.remaining_seconds = 60;
            let (h, _rx, tick_fut) = spawn_engine(config, state, 1000);
            let _tick_task = tokio::spawn(tick_fut);
            assert!(h.skip_rest(1000).await);
            let (st, idx, _) = h.get_status().await;
            assert_eq!(st, Status::Running);
            assert_eq!(idx, 3); // 跳到 work(3)
        });
    }

    #[test]
    fn reset_to_first_reload_config() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = cfg();
            let mut state = AppState::fresh_first_stage(60, 1000);
            state.current_stage_index = 1;
            let (h, _rx, tick_fut) = spawn_engine(config, state, 1000);
            let _tick_task = tokio::spawn(tick_fut);
            let mut new_cfg = cfg();
            new_cfg.stages[0].minutes = 2; // 改成 2 分钟
            h.reset_to_first(new_cfg, 5000).await;
            let (st, idx, rem) = h.get_status().await;
            assert_eq!(st, Status::Running);
            assert_eq!(idx, 0);
            assert_eq!(rem, 120); // 新配置 2 分钟 = 120 秒
        });
    }
}
