// config.rs — FocusLock 配置数据模型与持久化
//
// 阶段 1：数据模型与配置层
// - Scheme（方案）/ TimePeriod（时间段）/ Routine（作息表）/ Config 结构
// - 数据目录跨平台定位（dirs crate）
// - 读写 + 原子写 + 校验 + 非法回退默认
// - 配置改后不自动生效（保存只写文件，需「重置计时」或重启才生效）

use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

// ==================== 基础枚举 ====================

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

/// 休息提醒模式（时段结束动作用）
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

/// 音效类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundType {
    /// 无声（默认）
    None,
    /// 内置简单提示音（用 Web Audio API 生成）
    Builtin,
    /// 用户自定义音频文件（值为文件名，相对于 sounds/ 目录）
    Custom(String),
}

/// 时段结束动作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PeriodEndAction {
    /// 无动作
    None,
    /// 弹窗通知
    Popup {
        /// 提示文字（空字符串 = 不显示文字）
        text: String,
        /// 提示音
        sound: SoundType,
    },
    /// 全屏遮罩（半透明）
    Fullscreen {
        /// 提示文字
        text: String,
        /// 提示音
        sound: SoundType,
        /// 遮罩样式
        #[serde(default = "default_overlay_style")]
        style: OverlayStyle,
    },
    /// 全黑屏遮盖
    BlackScreen {
        /// 提示文字
        text: String,
        /// 提示音
        sound: SoundType,
    },
}

// ==================== 方案 / 时间段 / 作息表 ====================

/// 工作/休息循环方案（Scheme）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scheme {
    /// 方案唯一 ID
    pub id: String,
    /// 方案名称（如 "番茄工作法"、"标准45+15"）
    pub name: String,
    /// 阶段循环列表
    pub stages: Vec<Stage>,
    /// 休息提醒模式（本方案内休息时的提醒方式）
    #[serde(default = "default_rest_reminder_mode")]
    pub rest_reminder_mode: RestReminderMode,
    /// 工作结束（进入休息）提示音
    #[serde(default = "default_sound_none")]
    pub work_end_sound: SoundType,
    /// 休息结束（返回工作）提示音
    #[serde(default = "default_sound_builtin")]
    pub rest_end_sound: SoundType,
}

/// 一天中的一个时间段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePeriod {
    /// 唯一 ID
    pub id: String,
    /// 开始时间（小时 0-23）
    pub start_hour: u8,
    /// 开始时间（分钟 0-59）
    pub start_minute: u8,
    /// 结束时间（小时 0-23）
    pub end_hour: u8,
    /// 结束时间（分钟 0-59）
    pub end_minute: u8,
    /// 使用的方案 ID
    pub scheme_id: String,
    /// 时段结束时的动作
    pub end_action: PeriodEndAction,
}

/// 作息表（一天的时间段集合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Routine {
    /// 作息表唯一 ID
    pub id: String,
    /// 名称（如 "工作日"、"周末"）
    pub name: String,
    /// 时间段列表（按开始时间排序）
    pub periods: Vec<TimePeriod>,
}

/// 周配置：为周一~周日各分配一个作息表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyAssignment {
    pub monday: String,
    pub tuesday: String,
    pub wednesday: String,
    pub thursday: String,
    pub friday: String,
    pub saturday: String,
    pub sunday: String,
}

impl Default for WeeklyAssignment {
    fn default() -> Self {
        // 默认全部使用 "default" 作息表
        Self {
            monday:    "default".to_string(),
            tuesday:   "default".to_string(),
            wednesday:  "default".to_string(),
            thursday:   "default".to_string(),
            friday:     "default".to_string(),
            saturday:   "default".to_string(),
            sunday:     "default".to_string(),
        }
    }
}

// ==================== 顶层配置 ====================

/// 应用配置（v2 数据模型）
/// - stages：引擎当前使用的阶段列表（由调度器根据当前方案更新）
/// - schemes：用户定义的方案库
/// - routines：作息表（每天的时间段安排）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // ---- 引擎当前使用的阶段（调度器会根据当前方案更新此字段）----
    /// 当前生效的阶段循环列表（引擎直接读取此字段）
    pub stages: Vec<Stage>,

    // ---- 方案库与作息表 ----
    /// 工作/休息循环方案库
    #[serde(default = "default_schemes")]
    pub schemes: Vec<Scheme>,
    /// 作息表列表
    #[serde(default = "default_routines")]
    pub routines: Vec<Routine>,
    /// 周配置（每天用哪个作息表）
    #[serde(default)]
    pub weekly: WeeklyAssignment,

    // ---- 全局设置 ----
    /// 过夜/长时间离开重置阈值（分钟）
    #[serde(default = "default_reset_threshold")]
    pub reset_threshold_minutes: u32,
    /// 工作结束前 N 分钟发准备休息通知
    #[serde(default = "default_notify_before")]
    pub notify_before_work_end_minutes: u32,
    /// 跳过休息的全局快捷键
    #[serde(default = "default_skip_shortcut")]
    pub skip_shortcut: String,
    /// 是否以管理员权限自启（仅 Windows）
    #[serde(default)]
    pub run_as_admin_autostart: bool,
    /// 界面语言（"zh" / "en"）
    #[serde(default = "default_language")]
    pub language: String,
}

// ==================== 默认值函数 ====================

fn default_overlay_style() -> OverlayStyle {
    OverlayStyle::SemiTransparent
}

fn default_reset_threshold() -> u32 { 30 }
fn default_notify_before() -> u32 { 1 }
fn default_skip_shortcut() -> String { "CmdOrCtrl+Shift+F2".to_string() }
fn default_language() -> String { "zh".to_string() }

fn default_sound_none() -> SoundType { SoundType::None }
fn default_sound_builtin() -> SoundType { SoundType::Builtin }
fn default_rest_reminder_mode() -> RestReminderMode { RestReminderMode::Fullscreen }

/// 默认方案列表（含内置方案）
fn default_schemes() -> Vec<Scheme> {
    vec![
        Scheme {
            id: "pomodoro".to_string(),
            name: "番茄工作法".to_string(),
            stages: vec![
                Stage { stage_type: StageType::Work, minutes: 25 },
                Stage { stage_type: StageType::Rest, minutes: 5 },
                Stage { stage_type: StageType::Work, minutes: 25 },
                Stage { stage_type: StageType::Rest, minutes: 15 },
            ],
            rest_reminder_mode: RestReminderMode::Fullscreen,
            work_end_sound: SoundType::None,
            rest_end_sound: SoundType::Builtin,
        },
        Scheme {
            id: "standard".to_string(),
            name: "标准 45+15".to_string(),
            stages: vec![
                Stage { stage_type: StageType::Work, minutes: 45 },
                Stage { stage_type: StageType::Rest, minutes: 15 },
            ],
            rest_reminder_mode: RestReminderMode::Fullscreen,
            work_end_sound: SoundType::None,
            rest_end_sound: SoundType::Builtin,
        },
        Scheme {
            id: "deep".to_string(),
            name: "深度工作 50+10".to_string(),
            stages: vec![
                Stage { stage_type: StageType::Work, minutes: 50 },
                Stage { stage_type: StageType::Rest, minutes: 10 },
            ],
            rest_reminder_mode: RestReminderMode::Fullscreen,
            work_end_sound: SoundType::None,
            rest_end_sound: SoundType::Builtin,
        },
    ]
}

/// 默认作息表列表
fn default_routines() -> Vec<Routine> {
    vec![
        // 默认工作作息表
        Routine {
            id: "default".to_string(),
            name: "工作作息表".to_string(),
            periods: vec![
                TimePeriod {
                    id: "p1".to_string(),
                    start_hour: 8, start_minute: 0,
                    end_hour: 12, end_minute: 0,
                    scheme_id: "pomodoro".to_string(),
                    end_action: PeriodEndAction::Fullscreen {
                        text: "到饭点了！".to_string(),
                        sound: SoundType::Builtin,
                        style: OverlayStyle::SemiTransparent,
                    },
                },
                TimePeriod {
                    id: "p2".to_string(),
                    start_hour: 14, start_minute: 0,
                    end_hour: 18, end_minute: 0,
                    scheme_id: "standard".to_string(),
                    end_action: PeriodEndAction::Popup {
                        text: "工作日结束，休息一下吧！".to_string(),
                        sound: SoundType::Builtin,
                    },
                },
                TimePeriod {
                    id: "p3".to_string(),
                    start_hour: 19, start_minute: 0,
                    end_hour: 22, end_minute: 0,
                    scheme_id: "deep".to_string(),
                    end_action: PeriodEndAction::BlackScreen {
                        text: "睡觉啦".to_string(),
                        sound: SoundType::None,
                    },
                },
            ],
        },
        // 周末作息表
        Routine {
            id: "weekend".to_string(),
            name: "周末作息表".to_string(),
            periods: vec![
                TimePeriod {
                    id: "w1".to_string(),
                    start_hour: 9, start_minute: 0,
                    end_hour: 12, end_minute: 0,
                    scheme_id: "standard".to_string(),
                    end_action: PeriodEndAction::Popup {
                        text: "周末愉快！".to_string(),
                        sound: SoundType::Builtin,
                    },
                },
            ],
        },
    ]
}

impl Default for Config {
    fn default() -> Self {
        let default_schemes = default_schemes();
        let default_stages = default_schemes[0].stages.clone();
        Self {
            stages:   default_stages,
            schemes:   default_schemes,
            routines:  default_routines(),
            weekly:    WeeklyAssignment::default(),
            reset_threshold_minutes:       default_reset_threshold(),
            notify_before_work_end_minutes: default_notify_before(),
            skip_shortcut:                default_skip_shortcut(),
            run_as_admin_autostart:       false,
            language:                      default_language(),
        }
    }
}

// ==================== Config 方法 ====================

impl Config {
    /// 配置文件所在目录
    pub fn data_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("FocusLock")
    }

    pub fn config_path() -> PathBuf {
        Self::data_dir().join("config.json")
    }

    /// 校验配置合法性
    pub fn validate(&self) -> Result<(), String> {
        if self.schemes.is_empty() {
            return Err("至少需要一个方案".into());
        }
        for s in &self.schemes {
            if s.stages.is_empty() {
                return Err(format!("方案「{}」的阶段不能为空", s.name));
            }
            if !s.stages.iter().any(|st| st.stage_type == StageType::Work) {
                return Err(format!("方案「{}」至少需要一个工作阶段", s.name));
            }
            for st in &s.stages {
                if st.minutes == 0 {
                    return Err(format!("方案「{}」的阶段分钟数必须 > 0", s.name));
                }
            }
        }
        if self.routines.is_empty() {
            return Err("至少需要一个作息表".into());
        }
        // 检查 routine 引用的 scheme_id 是否存在
        let scheme_ids: std::collections::HashSet<&str> =
            self.schemes.iter().map(|s| s.id.as_str()).collect();
        for r in &self.routines {
            for p in &r.periods {
                if !scheme_ids.contains(p.scheme_id.as_str()) {
                    return Err(format!(
                        "作息表「{}」的时间段引用了不存在的方案 ID: {}",
                        r.name, p.scheme_id
                    ));
                }
            }
        }
        Ok(())
    }

    /// 从文件加载（含向后兼容迁移）
    pub fn load() -> (Config, bool) {
        let path = Self::config_path();
        match fs::read_to_string(&path) {
            Ok(content) => {
                // 尝试按新格式解析
                match serde_json::from_str::<Config>(&content) {
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
                        // 尝试按旧格式（v0.1.1）解析并迁移
                        if let Ok(old) = serde_json::from_str::<OldConfig>(&content) {
                            tracing::info!("检测到旧版配置，自动迁移到 v2 格式");
                            let new_cfg = old.migrate();
                            let _ = new_cfg.save();
                            (new_cfg, false)
                        } else {
                            tracing::warn!("配置 JSON 解析失败，回退默认: {}", e);
                            let def = Config::default();
                            let _ = def.save();
                            (def, true)
                        }
                    }
                }
            }
            Err(_) => {
                let def = Config::default();
                let _ = def.save();
                (def, false)
            }
        }
    }

    /// 原子写
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

// ==================== 旧配置格式（v0.1.1 兼容） ====================

#[derive(Deserialize)]
struct OldConfig {
    stages: Vec<Stage>,
    #[serde(default = "default_overlay_style_old")]
    overlay_style: OverlayStyle,
    #[serde(default = "default_rest_message_old")]
    rest_message: String,
    #[serde(default = "default_work_end_sound_old")]
    work_end_sound: SoundType,
    #[serde(default = "default_rest_end_sound_old")]
    rest_end_sound: SoundType,
    reset_threshold_minutes: u32,
    notify_before_work_end_minutes: u32,
    skip_shortcut: String,
    #[serde(default)]
    run_as_admin_autostart: bool,
    #[serde(default = "default_language_old")]
    language: String,
}

fn default_overlay_style_old() -> OverlayStyle { OverlayStyle::SemiTransparent }
fn default_rest_message_old() -> String { "现在休息".to_string() }
fn default_work_end_sound_old() -> SoundType { SoundType::None }
fn default_rest_end_sound_old() -> SoundType { SoundType::Builtin }
fn default_language_old() -> String { "zh".to_string() }

impl OldConfig {
    fn migrate(self) -> Config {
        // 将旧配置的阶段列表迁移为一个 "legacy" 方案
        let legacy_scheme = Scheme {
            id: "legacy".to_string(),
            name: "自定义方案".to_string(),
            stages: self.stages.clone(),
            rest_reminder_mode: RestReminderMode::Fullscreen,
            work_end_sound: self.work_end_sound.clone(),
            rest_end_sound: self.rest_end_sound.clone(),
        };
        let mut schemes = default_schemes();
        schemes.push(legacy_scheme.clone());

        // 创建一个使用 legacy 方案的默认作息表
        let default_routine = Routine {
            id: "default".to_string(),
            name: "默认作息表".to_string(),
            periods: vec![
                TimePeriod {
                    id: "p1".to_string(),
                    start_hour: 8, start_minute: 0,
                    end_hour: 12, end_minute: 0,
                    scheme_id: "legacy".to_string(),
                    end_action: PeriodEndAction::Fullscreen {
                        text: self.rest_message.clone(),
                        sound: self.rest_end_sound.clone(),
                        style: self.overlay_style,
                    },
                },
                TimePeriod {
                    id: "p2".to_string(),
                    start_hour: 14, start_minute: 0,
                    end_hour: 18, end_minute: 0,
                    scheme_id: "legacy".to_string(),
                    end_action: PeriodEndAction::Popup {
                        text: "工作日结束".to_string(),
                        sound: self.rest_end_sound.clone(),
                    },
                },
            ],
        };

        Config {
            stages: legacy_scheme.stages.clone(), // 使用 legacy 方案的 stages
            schemes,
            routines: vec![default_routine],
            weekly: WeeklyAssignment::default(),
            reset_threshold_minutes:       self.reset_threshold_minutes,
            notify_before_work_end_minutes: self.notify_before_work_end_minutes,
            skip_shortcut:                self.skip_shortcut,
            run_as_admin_autostart:       self.run_as_admin_autostart,
            language:                      self.language,
        }
    }
}

// ===== 作息表调度辅助方法 =====

impl WeeklyAssignment {
    /// 根据 chrono Weekday 获取对应的 routine_id
    pub fn get_routine_id(&self, weekday: chrono::Weekday) -> String {
        match weekday {
            chrono::Weekday::Mon => self.monday.clone(),
            chrono::Weekday::Tue => self.tuesday.clone(),
            chrono::Weekday::Wed => self.wednesday.clone(),
            chrono::Weekday::Thu => self.thursday.clone(),
            chrono::Weekday::Fri => self.friday.clone(),
            chrono::Weekday::Sat => self.saturday.clone(),
            chrono::Weekday::Sun => self.sunday.clone(),
        }
    }
}

impl Routine {
    /// 判断当前本地时间是否落在本作息表的某个时间段内，
    /// 返回 (period_index, &TimePeriod)；若无匹配返回 None。
    pub fn active_period(&self, now: NaiveTime) -> Option<(usize, &TimePeriod)> {
        for (i, p) in self.periods.iter().enumerate() {
            let start = NaiveTime::from_hms_opt(p.start_hour as u32, p.start_minute as u32, 0).unwrap();
            let end   = NaiveTime::from_hms_opt(p.end_hour   as u32, p.end_minute   as u32, 0).unwrap();
            if now >= start && now < end {
                return Some((i, p));
            }
        }
        None
    }
}

impl Config {
    /// 根据当前本地时间返回当前应使用的方案 ID 和当前时间段索引。
    /// 若当前不在任何时间段内，返回 None。
    pub fn resolve_current_schedule(&self, now: chrono::DateTime<chrono::Local>)
        -> Option<(String, usize)>
    {
        let routine_id = self.weekly.get_routine_id(now.weekday());
        let routine = self.routines.iter().find(|r| r.id == routine_id)?;
        let naive_time = now.time();
        let (period_idx, _period) = routine.active_period(naive_time)?;
        let scheme_id = routine.periods[period_idx].scheme_id.clone();
        Some((scheme_id, period_idx))
    }

    /// 返回当前时间所在的 TimePeriod 的结束时间（NaiveTime），
    /// 用于判断是否需要触发时段结束动作。
    /// 若不在任何时间段内，返回 None。
    pub fn current_period_end_time(&self, now: DateTime<Local>)
        -> Option<NaiveTime>
    {
        let routine_id = self.weekly.get_routine_id(now.weekday());
        let routine = self.routines.iter().find(|r| r.id == routine_id)?;
        let naive_time = now.time();
        let (_period_idx, period) = routine.active_period(naive_time)?;
        Some(NaiveTime::from_hms_opt(
            period.end_hour as u32, period.end_minute as u32, 0
        ).unwrap())
    }

    /// 返回下一个时间段的开始时间（用于等待切换），
    /// 以及该时间段使用的方案 ID。
    /// 若当前在所有时间段之外，返回今天下一个时间段的开始时间；
    /// 若今天没有更多时间段，返回明天第一个时间段的开始时间。
    pub fn next_period_start(&self, now: DateTime<Local>)
        -> Option<(DateTime<Local>, String)>
    {
        let routine_id = self.weekly.get_routine_id(now.weekday());
        let routine = self.routines.iter().find(|r| r.id == routine_id)?;
        let naive_time = now.time();

        // 找今天剩下的时间段
        for p in &routine.periods {
            let start = NaiveTime::from_hms_opt(p.start_hour as u32, p.start_minute as u32, 0).unwrap();
            if start > naive_time {
                let naive_dt = NaiveDateTime::new(now.date_naive(), start);
                if let Some(local_dt) = Local.from_local_datetime(&naive_dt).single() {
                    return Some((local_dt, p.scheme_id.clone()));
                }
            }
        }
        // 今天没有更多了，找明天第一个
        let next_day = now + Duration::days(1);
        let next_routine_id = self.weekly.get_routine_id(next_day.weekday());
        let next_routine = self.routines.iter().find(|r| r.id == next_routine_id)?;
        if let Some(p) = next_routine.periods.first() {
            let start = NaiveTime::from_hms_opt(p.start_hour as u32, p.start_minute as u32, 0).unwrap();
            let naive_dt = NaiveDateTime::new(next_day.date_naive(), start);
            if let Some(local_dt) = Local.from_local_datetime(&naive_dt).single() {
                return Some((local_dt, p.scheme_id.clone()));
            }
        }
        None
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = Config::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.schemes.len(), back.schemes.len());
    }

    #[test]
    fn old_config_migrates() {
        let old = OldConfig {
            stages: vec![
                Stage { stage_type: StageType::Work, minutes: 45 },
                Stage { stage_type: StageType::Rest, minutes: 15 },
            ],
            overlay_style: OverlayStyle::SemiTransparent,
            rest_message: "现在休息".to_string(),
            work_end_sound: SoundType::None,
            rest_end_sound: SoundType::Builtin,
            reset_threshold_minutes: 30,
            notify_before_work_end_minutes: 1,
            skip_shortcut: "CmdOrCtrl+Shift+F2".to_string(),
            run_as_admin_autostart: false,
            language: "zh".to_string(),
        };
        let new_cfg = old.migrate();
        assert!(new_cfg.validate().is_ok());
        assert!(new_cfg.schemes.iter().any(|s| s.id == "legacy"));
    }
}
