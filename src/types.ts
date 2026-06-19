// 前端类型定义，与后端 config.rs 的 serde 序列化对齐
//
// 注意：StageType 用 lowercase 序列化（"work"/"rest"），
// RestReminderMode 同理（"fullscreen"/"popup"）
// OverlayStyle 用 snake_case 序列化（"semi_transparent"/"full_black"/"dark"）

export type StageType = "work" | "rest";
export type RestReminderMode = "fullscreen" | "popup";
export type OverlayStyle = "semi_transparent" | "full_black" | "dark";

export interface Stage {
  type: StageType;
  minutes: number;
}

export interface Config {
  stages: Stage[];
  rest_reminder_mode: RestReminderMode;
  overlay_style: OverlayStyle;
  rest_message: string;
  reset_threshold_minutes: number;
  notify_before_work_end_minutes: number;
  skip_shortcut: string;
  run_as_admin_autostart: boolean;
}
