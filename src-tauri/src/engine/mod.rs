// engine 模块入口
//
// 计时引擎核心（实施计划阶段 2）：
// - timer.rs：tokio 后台 tick + 状态机 + 推进逻辑
// - stage.rs：阶段循环游标（纯逻辑）
// - reset.rs：过夜/长时间离开重置判定（纯函数，供启动流程与电源事件复用）
//
// 引擎不直接依赖 Tauri，可独立测试。
// 与 Tauri 的桥接（命令暴露、事件 emit、通知发送）在 lib.rs / commands.rs 层完成。

pub mod reset;
pub mod stage;
pub mod timer;

pub use reset::{check_and_reset, ResetVerdict};
pub use timer::{spawn_engine, EngineEvent, EngineHandle};
