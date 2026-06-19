// engine/reset.rs — 过夜/长时间离开重置判定
//
// 双触发机制（实施计划阶段 8）：
//   A. 系统电源事件（Win: WM_POWERBROADCAST；Mac: NSWorkspaceDidWakeNotification）
//   B. 定时器时间跳跃兜底（已在 timer.rs 的 tick 循环中实现：delta > 阈值则重置）
//
// 本模块提供：
//   - check_and_reset 纯函数：给定状态、当前时间、阈值，判定是否需要重置
//   - 供 timer.rs tick 与电源事件回调共用
//
// 核心排除条件：暂停状态不触发重置（即使暂停一整天）。
//
// 注意：timer.rs 的 tick 循环已内联了重置逻辑（直接 enter_stage(0)），
// 本模块的纯函数主要用于：
//   1. 启动时一次性判定（lib.rs 启动流程）
//   2. 单元测试可独立验证判定逻辑
//   3. 电源事件回调时复用

use crate::state::{AppState, Status};

/// 重置判定结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResetVerdict {
    /// 不重置
    No,
    /// 需要重置（时间跳跃超过阈值）
    Yes,
}

/// 判定是否需要重置
///
/// - `state` 当前持久化状态
/// - `now` 当前 Unix 时间戳（秒）
/// - `threshold_secs` 重置阈值（秒）
///
/// 规则：
/// 1. status == Paused → No（核心排除）
/// 2. delta = now - last_active_timestamp；delta <= 0 → No（时间往回拨，按 0 处理）
/// 3. delta > threshold_secs → Yes
/// 4. 否则 No
pub fn check_and_reset(state: &AppState, now: i64, threshold_secs: u64) -> ResetVerdict {
    if state.status == Status::Paused {
        return ResetVerdict::No;
    }
    let delta = now - state.last_active_timestamp;
    if delta <= 0 {
        return ResetVerdict::No;
    }
    if (delta as u64) > threshold_secs {
        ResetVerdict::Yes
    } else {
        ResetVerdict::No
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(status: Status, last_active: i64, remaining: u64) -> AppState {
        AppState {
            current_stage_index: 1,
            status,
            last_active_timestamp: last_active,
            remaining_seconds: remaining,
        }
    }

    #[test]
    fn paused_never_resets_even_if_huge_delta() {
        let s = state(Status::Paused, 1000, 300);
        assert_eq!(check_and_reset(&s, 100000, 1800), ResetVerdict::No);
    }

    #[test]
    fn running_resets_when_delta_exceeds_threshold() {
        let s = state(Status::Running, 1000, 300);
        // delta = 2000 - 1000 = 1000 < 1800 → No
        assert_eq!(check_and_reset(&s, 2000, 1800), ResetVerdict::No);
        // delta = 3000 - 1000 = 2000 > 1800 → Yes
        assert_eq!(check_and_reset(&s, 3000, 1800), ResetVerdict::Yes);
    }

    #[test]
    fn resting_also_subject_to_reset() {
        let s = state(Status::Resting, 1000, 300);
        assert_eq!(check_and_reset(&s, 3000, 1800), ResetVerdict::Yes);
    }

    #[test]
    fn time_going_backward_does_not_reset() {
        // 用户手动把系统时间往回拨
        let s = state(Status::Running, 3000, 300);
        assert_eq!(check_and_reset(&s, 1000, 1800), ResetVerdict::No);
    }

    #[test]
    fn exactly_at_threshold_does_not_reset() {
        // delta == threshold 不重置（严格大于）
        let s = state(Status::Running, 1000, 300);
        assert_eq!(check_and_reset(&s, 2800, 1800), ResetVerdict::No); // delta=1800
        assert_eq!(check_and_reset(&s, 2801, 1800), ResetVerdict::Yes); // delta=1801
    }
}
