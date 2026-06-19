// config.rs — FocusLock 配置数据模型与持久化
//
// 阶段 1：数据模型与配置层
// - Config / Stage / StageType / RestReminderMode 结构
// - 数据目录跨平台定位（dirs crate）
// - 读写 + 原子写 + 校验 + 非法回退默认
// - 配置改后不自动生效（保存只写文件，需「重置计时」或重启才生效，由引擎层处理）

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// 阶段类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageType {
    Work,
    Rest,
}

/// 单个阶段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    /// 阶段类型，JSON 字段名为 "type"
    #[serde(rename = "type")]
    pub stage_type: StageType,
    /// 持续时间（分钟）
    pub minutes: u32,
}

/// 休息提醒模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RestReminderMode {
    /// 全屏遮罩（默认）
    Fullscreen,
    /// 右下角弹窗
    Popup,
}

/// 遮罩样式（仅 fullscreen 模式生效）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayStyle {
    /// 半透明黑底（默认，当前效果）
    SemiTransparent,
    /// 纯黑不透明
    FullBlack,
    /// 深色暗调
    Dark,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 阶段循环列表
    pub stages: Vec<Stage>,
    /// 休息提醒模式
    pub rest_reminder_mode: RestReminderMode,
    /// 遮罩样式（仅 fullscreen 模式生效）
    #[serde(default = "default_overlay_style")]
    pub overlay_style: OverlayStyle,
    /// 休息遮罩显示的自定义提示词
    #[serde(default = "default_rest_message")]
    pub rest_message: String,
    /// 过夜/长时间离开重置阈值（分钟）
    pub reset_threshold_minutes: u32,
    /// 工作结束前 N 分钟发准备休息通知
    pub notify_before_work_end_minutes: u32,
    /// 跳过休息的全局快捷键（Tauri accelerator 格式，跨平台 CmdOrCtrl 自动映射）
    pub skip_shortcut: String,
    /// 是否以管理员权限自启（仅 Windows 生效，macOS 忽略）
    #[serde(default)]
    pub run_as_admin_autostart: bool,
}

fn default_overlay_style() -> OverlayStyle {
    OverlayStyle::SemiTransparent
}

fn default_rest_message() -> String {
    "现在休息".to_string()
}

impl Default for Config {
    fn default() -> Self {
        // 默认配置：工作 45 / 休息 15，fullscreen，半透明遮罩，阈值 30 分钟
        Self {
            stages: vec![
                Stage {
                    stage_type: StageType::Work,
                    minutes: 45,
                },
                Stage {
                    stage_type: StageType::Rest,
                    minutes: 15,
                },
            ],
            rest_reminder_mode: RestReminderMode::Fullscreen,
            overlay_style: OverlayStyle::SemiTransparent,
            rest_message: "现在休息".to_string(),
            reset_threshold_minutes: 30,
            notify_before_work_end_minutes: 1,
            skip_shortcut: "CmdOrCtrl+Shift+F2".to_string(),
            run_as_admin_autostart: false,
        }
    }
}

impl Config {
    /// 配置文件所在目录：Win=%APPDATA%/FocusLock，Mac=~/Library/Application Support/FocusLock
    pub fn data_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("FocusLock")
    }

    /// 配置文件路径
    pub fn config_path() -> PathBuf {
        Self::data_dir().join("config.json")
    }

    /// 校验配置合法性
    pub fn validate(&self) -> Result<(), String> {
        if self.stages.is_empty() {
            return Err("stages 不能为空".into());
        }
        if !self.stages.iter().any(|s| s.stage_type == StageType::Work) {
            return Err("stages 至少需要包含一个 work 阶段".into());
        }
        for s in &self.stages {
            if s.minutes == 0 {
                return Err("每个阶段 minutes 必须 > 0".into());
            }
        }
        if self.reset_threshold_minutes == 0 {
            return Err("reset_threshold_minutes 必须 > 0".into());
        }
        if self.skip_shortcut.trim().is_empty() {
            return Err("skip_shortcut 不能为空".into());
        }
        Ok(())
    }

    /// 从文件加载；若文件不存在则生成默认并保存；若 JSON 非法或校验失败则回退默认
    /// 返回 (config, 回退标志) —— 回退为 true 时调用方应发通知提醒用户
    pub fn load() -> (Config, bool) {
        let path = Self::config_path();
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Config>(&content) {
                Ok(cfg) => match cfg.validate() {
                    Ok(()) => (cfg, false),
                    Err(e) => {
                        tracing::warn!("配置校验失败，回退默认: {}", e);
                        let def = Config::default();
                        let _ = def.save();
                        (def, true)
                    }
                },
                Err(e) => {
                    tracing::warn!("配置 JSON 解析失败，回退默认: {}", e);
                    let def = Config::default();
                    let _ = def.save();
                    (def, true)
                }
            },
            Err(_) => {
                // 文件不存在，首次启动，生成默认
                let def = Config::default();
                let _ = def.save();
                (def, false)
            }
        }
    }

    /// 原子写：先写临时文件再 rename，避免断电损坏
    pub fn save(&self) -> std::io::Result<()> {
        let dir = Self::data_dir();
        fs::create_dir_all(&dir)?;
        let path = Self::config_path();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn empty_stages_invalid() {
        let mut cfg = Config::default();
        cfg.stages.clear();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn no_work_stage_invalid() {
        let cfg = Config {
            stages: vec![Stage {
                stage_type: StageType::Rest,
                minutes: 5,
            }],
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn zero_minutes_invalid() {
        let mut cfg = Config::default();
        cfg.stages[0].minutes = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = Config::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.stages.len(), back.stages.len());
        assert_eq!(back.stages[0].stage_type, StageType::Work);
    }

    #[test]
    fn stage_type_serializes_lowercase() {
        let s = serde_json::to_string(&StageType::Work).unwrap();
        assert_eq!(s, "\"work\"");
        let t: StageType = serde_json::from_str("\"rest\"").unwrap();
        assert_eq!(t, StageType::Rest);
    }
}
