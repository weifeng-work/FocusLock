// engine/stage.rs — 阶段定义与循环推进逻辑
//
// 纯逻辑模块，不依赖 IO 与时间，便于单元测试。
// - 阶段推进：index = (index + 1) % stages.len()
// - 跳过休息：向后扫描找到下一个 type=work 的阶段
// - 当前阶段总秒数

use crate::config::{Stage, StageType};

/// 阶段循环游标工具
pub struct StageCursor<'a> {
    pub stages: &'a [Stage],
    pub index: usize,
}

impl<'a> StageCursor<'a> {
    pub fn new(stages: &'a [Stage], index: usize) -> Self {
        Self { stages, index }
    }

    /// 当前阶段
    pub fn current(&self) -> &Stage {
        &self.stages[self.index]
    }

    /// 当前阶段总秒数
    pub fn current_total_seconds(&self) -> u64 {
        u64::from(self.current().minutes) * 60
    }

    /// 推进到下一阶段（循环）。返回新 index。
    /// 用于「休息自然结束」或「工作自然结束」。
    pub fn advance(&self) -> usize {
        (self.index + 1) % self.stages.len()
    }

    /// 跳过当前休息阶段，进入「下一个 work 阶段」。
    /// 向后扫描 stages，找到第一个 type=work 的位置。
    /// 若配置非法（无 work，已在 config 校验拦截），兜底返回 0。
    /// 返回 (新 index, 是否跳过了中间阶段)
    pub fn skip_rest_to_next_work(&self) -> (usize, bool) {
        let n = self.stages.len();
        for offset in 1..=n {
            let i = (self.index + offset) % n;
            if self.stages[i].stage_type == StageType::Work {
                // offset == 1 表示紧邻下一阶段就是 work，没有"跳过"中间阶段
                return (i, offset > 1);
            }
        }
        // 兜底：理论上不会到达（config 校验保证至少一个 work）
        (0, self.index != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(m: u32) -> Stage {
        Stage {
            stage_type: StageType::Work,
            minutes: m,
        }
    }
    fn rest(m: u32) -> Stage {
        Stage {
            stage_type: StageType::Rest,
            minutes: m,
        }
    }

    #[test]
    fn advance_wraps_around() {
        let stages = vec![work(45), rest(15)];
        assert_eq!(StageCursor::new(&stages, 0).advance(), 1);
        assert_eq!(StageCursor::new(&stages, 1).advance(), 0);
    }

    #[test]
    fn current_total_seconds() {
        let stages = vec![work(45), rest(15)];
        assert_eq!(StageCursor::new(&stages, 0).current_total_seconds(), 2700);
        assert_eq!(StageCursor::new(&stages, 1).current_total_seconds(), 900);
    }

    #[test]
    fn skip_rest_when_next_is_work() {
        // [work, rest] 当前在 rest(1)，跳过应到 work(0)，未跳过中间
        let stages = vec![work(45), rest(15)];
        let (i, skipped) = StageCursor::new(&stages, 1).skip_rest_to_next_work();
        assert_eq!(i, 0);
        assert!(!skipped);
    }

    #[test]
    fn skip_rest_jumps_consecutive_rests() {
        // [work, rest, rest, work] 当前在 rest(1)，跳过应跳过 rest(2) 到 work(3)
        let stages = vec![work(25), rest(5), rest(5), work(25)];
        let (i, skipped) = StageCursor::new(&stages, 1).skip_rest_to_next_work();
        assert_eq!(i, 3);
        assert!(skipped);
    }

    #[test]
    fn skip_rest_wraps_around() {
        // [work, rest, rest] 当前在 rest(2)，下一个 work 在 0（回环）
        let stages = vec![work(45), rest(10), rest(10)];
        let (i, skipped) = StageCursor::new(&stages, 2).skip_rest_to_next_work();
        assert_eq!(i, 0);
        assert!(!skipped); // offset=1，紧邻回环不算跳过中间
    }
}
