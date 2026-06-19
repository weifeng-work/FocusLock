// state.rs — FocusLock 运行状态持久化
//
// 阶段 1：状态文件（state.json）用于断电/异常重启后恢复
// - status: Running / Paused / Resting
// - 启动恢复策略：
//   * 有 state 且时间跳跃 < 阈值 → 恢复状态继续
//   * 时间跳跃 >= 阈值（且非 Paused）→ 重置到阶段 0
//   * Paused 状态重启 → 自动恢复 Running（从冻结 remaining 继续）—— 见阶段 9
// - 暂停期间不触发过夜重置（由引擎层判定，state 仅记录）

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// 运行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// 工作阶段倒计时中
    Running,
    /// 已暂停，倒计时冻结
    Paused,
    /// 休息阶段（遮罩/弹窗显示中）
    Resting,
}

/// 持久化状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    /// 当前在 stages 数组中的索引
    pub current_stage_index: usize,
    /// 当前状态
    pub status: Status,
    /// 上次更新的 Unix 时间戳（秒）
    pub last_active_timestamp: i64,
    /// 当前阶段剩余的秒数
    pub remaining_seconds: u64,
}

impl AppState {
    /// state 文件路径（与 config 同目录）
    pub fn state_path() -> PathBuf {
        crate::config::Config::data_dir().join("state.json")
    }

    /// 从文件加载；文件不存在或损坏返回 None（调用方按新启动处理）
    pub fn load() -> Option<AppState> {
        let path = Self::state_path();
        let content = fs::read_to_string(&path).ok()?;
        match serde_json::from_str::<AppState>(&content) {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!("state.json 解析失败，当作新启动: {}", e);
                None
            }
        }
    }

    /// 原子写：先写临时文件再 rename
    pub fn save(&self) -> std::io::Result<()> {
        let dir = crate::config::Config::data_dir();
        fs::create_dir_all(&dir)?;
        let path = Self::state_path();
        let tmp = path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(content.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// 初始化为第一阶段开始
    pub fn fresh_first_stage(stage_seconds: u64, now_timestamp: i64) -> Self {
        Self {
            current_stage_index: 0,
            status: Status::Running,
            last_active_timestamp: now_timestamp,
            remaining_seconds: stage_seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_serde_lowercase() {
        assert_eq!(serde_json::to_string(&Status::Running).unwrap(), "\"running\"");
        assert_eq!(
            serde_json::to_string(&Status::Paused).unwrap(),
            "\"paused\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Resting).unwrap(),
            "\"resting\""
        );
        let s: Status = serde_json::from_str("\"resting\"").unwrap();
        assert_eq!(s, Status::Resting);
    }

    #[test]
    fn fresh_first_stage_is_running_at_index_zero() {
        let s = AppState::fresh_first_stage(2700, 1698240000);
        assert_eq!(s.current_stage_index, 0);
        assert_eq!(s.status, Status::Running);
        assert_eq!(s.remaining_seconds, 2700);
        assert_eq!(s.last_active_timestamp, 1698240000);
    }

    #[test]
    fn appstate_roundtrip() {
        let s = AppState {
            current_stage_index: 2,
            status: Status::Paused,
            last_active_timestamp: 1698240000,
            remaining_seconds: 123,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.current_stage_index, 2);
        assert_eq!(back.status, Status::Paused);
        assert_eq!(back.remaining_seconds, 123);
    }
}
