// 前端类型定义，与后端 config.rs 的 serde 序列化对齐
//
// 注意：StageType 用 lowercase 序列化（"work"/"rest"），
// RestReminderMode 同理（"fullscreen"/"popup"）
// OverlayStyle 用 snake_case 序列化（"semi_transparent"/"full_black"/"dark"）
// SoundType 用 snake_case 序列化（"none"/"builtin"/{"custom": "文件名"}）

export type StageType = "work" | "rest";
export type RestReminderMode = "fullscreen" | "popup";
export type OverlayStyle = "semi_transparent" | "full_black" | "dark";
export type SoundType = "none" | "builtin" | { custom: string };
export type RestMessageMode = "random" | "fixed";

export interface Stage {
  type: StageType;
  minutes: number;
}

// 时段结束动作
export type PeriodEndAction =
  | { type: "none" }
  | { type: "popup"; text: string; sound: SoundType }
  | { type: "fullscreen"; text: string; sound: SoundType; style: OverlayStyle }
  | { type: "black_screen"; text: string; sound: SoundType };

// 方案（Scheme）
export interface Scheme {
  id: string;
  name: string;
  stages: Stage[];
  rest_reminder_mode: RestReminderMode;
  work_end_sound: SoundType;
  rest_end_sound: SoundType;
}

// 时间段
export interface TimePeriod {
  id: string;
  start_hour: number;
  start_minute: number;
  end_hour: number;
  end_minute: number;
  scheme_id: string;
  end_action: PeriodEndAction;
}

// 作息表
export interface Routine {
  id: string;
  name: string;
  periods: TimePeriod[];
}

// 周配置
export interface WeeklyAssignment {
  monday: string;
  tuesday: string;
  wednesday: string;
  thursday: string;
  friday: string;
  saturday: string;
  sunday: string;
}

// 配置（v2 数据模型）
export interface Config {
  stages: Stage[]; // 引擎当前使用的阶段列表
  schemes: Scheme[];
  routines: Routine[];
  weekly: WeeklyAssignment;
  reset_threshold_minutes: number;
  notify_before_work_end_minutes: number;
  skip_shortcut: string;
  run_as_admin_autostart: boolean;
  language: string;
  /** 遮罩不透明度 0-100，默认 95 */
  overlay_opacity: number;
  /** 用户自定义休息提示文案；空时使用内置 10 条随机 */
  rest_messages: string[];
  /** 随机 / 固定轮询 */
  rest_message_mode: RestMessageMode;
  /** 内置提示音变体（alarm/chime/digital/pulse/bird） */
  builtin_sound_variant: string;
}

// 音效文件信息
export interface SoundFile {
  name: string;  // 显示名称
  file: string;  // 文件名（相对于 sounds/ 目录）
  path: string;  // 完整路径（用于播放）
}
