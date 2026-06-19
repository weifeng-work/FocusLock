# FocusLock 实施计划

> 一款「自定义循环，到点即强休」的跨平台轻量级专注助手
> 技术栈：Tauri 2.x（Rust 核心 + Vue 3 / TS 前端）
> 文档版本：v2.1　更新日期：2026-06-19
> v2.0 变更：跨平台（Win+Mac）；快捷键跳过休息；配置改后需手动重置/重启才生效；可选管理员自启；交互流程细化。
> v2.1 变更（第二轮确认结论）：Mac 遮罩用无边框置顶窗口覆盖屏幕；跳过休息→进入下一个 work 阶段；暂停状态重启后自动恢复 Running；快捷键 v1 即支持配置界面自定义；Linux 不纳入 v1；自动更新 v1 不做。

---

## 一、项目目标与范围

### 1.1 一句话定义
开机自启、后台静默的专注节奏管理工具。通过自定义「工作 / 休息」阶段循环，到点以全屏遮罩或弹窗强休，帮你守住节奏。

### 1.2 目标平台
- **Windows 10/11**（主力，x64）
- **macOS 12+**（Apple Silicon + Intel）
- Linux：暂不在 v1 范围内，但 Tauri 天然支持，架构上预留不阻断。

### 1.3 核心价值
- **轻量**：Tauri 体积小、内存低（目标常驻 < 60MB）。
- **可靠**：基于系统时间戳计时，断电/睡眠后可恢复。
- **可配置**：阶段顺序、时长、提醒方式全部可自定义。
- **软强制**：休息遮罩视觉阻断但不屏蔽系统快捷键，体感克制。
- **跨平台一致**：同一套核心逻辑，平台差异封装在抽象层后。

### 1.4 不做什么（边界）
- 不做任务管理、番茄统计、云端同步。
- 不做硬强制（不屏蔽 Alt+Tab / Win / Cmd+Tab 等系统快捷键）。
- 不处理覆盖管理员权限程序（如任务管理器）的极端越权场景（仅文档说明）。
- 不做版本自动更新（v1 手动更新，后续可加 Tauri updater）。

---

## 二、技术栈与工具链

| 层 | 选型 | 说明 |
|---|---|---|
| 核心逻辑 | Rust（stable） | 计时引擎、状态机、系统调用、文件 IO |
| 应用框架 | Tauri 2.x | 跨平台壳 |
| 前端框架 | Vue 3 + Vite + TypeScript（**已确认默认**） | 配置面板与遮罩 UI |
| 前端 UI | Naive UI（配置面板） + 纯 CSS（遮罩大字） | |
| 异步运行时 | tokio | 后台计时循环、事件订阅 |
| 序列化 | serde + serde_json | config / state |
| 系统调用 | 平台抽象 trait + 各平台后端 | 见 §2.2 |
| 日志 | tracing + tracing-subscriber | 结构化日志 |

### 2.1 关键 Tauri 插件
- `tauri-plugin-notification` — 系统通知（macOS 需请求授权）
- `tauri-plugin-autostart` — 开机自启（Win 写注册表 / Mac 写 launchd 登录项）
- `tauri-plugin-global-shortcut` — 全局快捷键（跳过休息）
- `tauri-plugin-dialog` — 异常提示弹窗
- `tauri-plugin-single-instance` — 单实例锁（跨平台）

### 2.2 平台抽象层（跨平台关键）
所有平台相关能力收敛到 Rust trait，业务逻辑只依赖 trait，不直接调平台 API：

| 能力 | Windows 实现 | macOS 实现 | 抽象接口 |
|---|---|---|---|
| 多显示器枚举 | `EnumDisplayMonitors` (windows-rs) | `NSScreen.screens` (cocoa/objc) | `MonitorApi::list_monitors() -> Vec<MonitorRect>` |
| 电源/唤醒事件 | `WM_POWERBROADCAST` | `NSWorkspaceDidWakeNotification` | `PowerApi::on_wake(callback)` |
| 全屏遮罩窗口 | WebviewWindow + 手动置顶 | WebviewWindow + `.level()` (floating/status) | 统一 `OverlayWindowManager` |
| 通知 | Windows Toast (via plugin) | UNUserNotificationCenter (via plugin) | `NotifyApi::send(...)` |
| 自启 | 注册表 Run 键 | launchd `~/Library/LaunchAgents` | `AutostartApi::enable/disable` |
| 数据目录 | `%APPDATA%/FocusLock` | `~/Library/Application Support/FocusLock` | `dirs` crate |
| 管理员/提权 | 可选：注册表 + UAC（见 §2.3） | 无对应概念，跳过 | — |

### 2.3 管理员权限自启（**已确认提供可选**）
- **仅 Windows**：在配置界面提供「以管理员权限自启（提升遮罩覆盖能力）」开关。
- 开启时：写注册表自启项指向一个带 `requireAdministrator` manifest 的启动器（或主程序自身），代价是每次开机弹一次 UAC。
- 关闭时（默认）：普通权限自启，遮罩无法覆盖任务管理器等管理员程序。
- **macOS**：无对应概念，该选项不显示。文档说明 Mac 上遮罩覆盖能力受 Mission Control/Space 限制（见风险表）。

---

## 三、项目结构

```
focuslock/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs
│       ├── config.rs              # Config 结构 + 读写（改后不自动生效）
│       ├── state.rs               # State 结构 + 持久化
│       ├── engine/
│       │   ├── mod.rs
│       │   ├── timer.rs           # 后台计时循环
│       │   ├── stage.rs           # 阶段推进
│       │   └── reset.rs           # 过夜重置
│       ├── platform/              # 平台抽象层
│       │   ├── mod.rs             # trait 定义
│       │   ├── windows/           # #[cfg(windows)] 后端
│       │   └── macos/             # #[cfg(target_os="macos")] 后端
│       ├── overlay.rs             # 遮罩窗口管理（跨平台）
│       ├── tray.rs
│       ├── notify.rs
│       ├── shortcut.rs            # 全局快捷键（跳过休息）
│       └── commands.rs            # Tauri command
├── src/                           # Vue 前端
│   ├── views/Settings.vue
│   ├── views/Overlay.vue
│   └── components/...
├── package.json
└── README.md
```

---

## 四、分阶段实施路线

### 阶段 0｜环境与脚手架
- [ ] Rust + Tauri CLI 2.x + Node + WebView2（Win）/ Xcode CLT（Mac）
- [ ] `create-tauri-app`（Vue + TS）
- [ ] tauri.conf.json：默认隐藏窗口、单实例锁
- [ ] 引入 autostart / notification / global-shortcut / single-instance 插件
- [ ] 建立目录与平台抽象 trait 占位
- [ ] Windows 与 macOS 均能 `tauri dev` 启动
- **验收**：双平台启动后无窗口、托盘可见。

### 阶段 1｜数据模型与配置层
- [ ] Config / State 结构（serde）
- [ ] 数据目录用 `dirs` crate 跨平台定位
- [ ] 默认配置：工作 45 / 休息 15，fullscreen，阈值 30 分钟
- [ ] 配置校验 + 非法回退默认 + 通知
- [ ] **配置改后不自动生效**（已确认）：保存即写文件，但引擎不热重载；UI 保存时弹提示「配置已保存，需重置计时或重启应用后生效」
- [ ] State 原子写（临时文件 + rename）
- **验收**：单元测试覆盖读写、非法回退、原子写。

### 阶段 2｜计时引擎核心
- [ ] tokio 后台 task，每秒 tick
- [ ] **基于系统时间戳推进**（非 sleep 减一）
- [ ] 阶段循环推进 `index = (index+1) % len`
- [ ] 状态机 Running ⇄ Resting，Pausing 可从两者进入
- [ ] 暂停冻结 remaining；恢复重设 started_at；**暂停期不触发重置**
- [ ] 每 60s + 状态切换写 state.json
- [ ] 工作结束前 N 分钟发「准备休息」事件
- [ ] Tauri command：`get_status` / `pause` / `resume` / `skip_stage` / `reset_to_first`
- **验收**：模拟时钟单元测试覆盖循环、暂停恢复、连续 rest、索引回环、从 state 恢复连续。

### 阶段 3｜系统托盘
- [ ] 三状态图标（工作中/休息中/暂停）
- [ ] 悬停 tooltip 实时倒计时
- [ ] 右键菜单：查看状态、暂停/恢复、**重置计时（重新加载配置并从阶段0开始）**、打开配置、退出
- [ ] 双击打开配置窗口
- [ ] macOS：menu bar item 行为适配（左键直接展开菜单）
- **验收**：双平台三状态图标切换、tooltip、菜单可用。

### 阶段 4｜系统通知
- [ ] 工作开始 / 准备休息（含「暂停循环」按钮）/ 重置通知
- [ ] **macOS 首次启动请求通知授权**；被拒后降级为托盘 tooltip + 配置窗口红点提示
- [ ] 通知按钮回调 → pause command
- **验收**：双平台通知触发；按钮暂停生效；Mac 授权流程顺畅。

### 阶段 5｜休息遮罩 UI（多显示器）⭐高风险
- [ ] `MonitorApi` 跨平台实现
- [ ] fullscreen 模式：
  - 主显示器 WebviewWindow 加载 Overlay.vue（巨大倒计时 + 小字快捷键提示）
  - 副显示器黑半透明无边框置顶窗口
  - 属性：always_on_top / decorations:false / transparent / 位置尺寸=显示器
  - 拦截 Webview 内点击，不屏蔽系统快捷键
  - 防失焦置顶（定时 re-raise 或失焦监听）
  - **macOS（已确认方案）**：使用无边框置顶窗口覆盖屏幕区域，**不**使用原生 fullscreen（避免进入独立 Space）。窗口 level 设为 floating/status；监听 Space/Mission Control 切换事件，切换后重新展示遮罩
- [ ] popup 模式：右下角小窗口
- [ ] **热插拔检测**：遮罩期间每 3s 重新枚举，新增即补罩
- [ ] Overlay.vue：大字倒计时 + **小字提示「按 Ctrl+Shift+F2 跳过休息」**（macOS 显示 Cmd+Shift+F2）
- [ ] 休息结束关闭所有遮罩
- **验收**：双屏覆盖 + 热插拔 + 归零自动关闭 + 快捷键提示可见。

### 阶段 6｜配置界面
- [ ] Settings.vue + StageEditor.vue：增删/拖拽排序/改类型/改分钟
- [ ] rest_reminder_mode / reset_threshold / notify_before_work_end 输入
- [ ] **Windows 专属**：管理员权限自启开关
- [ ] 保存 → 校验 → 写 config.json → **提示「需重置计时或重启生效」**（不自动热重载）
- [ ] 单例窗口
- **验收**：保存后文件正确；引擎按既定策略不立即生效；重置计时后按新配置运行。

### 阶段 7｜全局快捷键与跳过休息
- [ ] `tauri-plugin-global-shortcut` 注册跳过快捷键，**默认 Ctrl+Shift+F2（Win）/ Cmd+Shift+F2（Mac），可在配置界面自定义**（已确认 v1 即支持）
- [ ] 配置界面提供快捷键录入控件 + 冲突检测（录入时尝试注册，失败提示占用）
- [ ] 仅在 Resting 状态下生效；按下 → 关闭遮罩 → **进入下一个 work 阶段**（已确认）
- [ ] 跳过实现：从当前 index 向后扫描 stages，找到下一个 type=work 的阶段并跳到该 index；若循环中无 work（非法配置，校验阶段已拦截）则兜底进阶段 0
- [ ] 跳过时发「已跳过休息，进入工作阶段」通知
- **验收**：休息中按快捷键立即跳到下一个 work；工作中按无反应；自定义快捷键生效；占用时有提示。

### 阶段 8｜过夜/长时间离开重置
- [ ] `PowerApi`：Win=WM_POWERBROADCAST，Mac=NSWorkspaceDidWakeNotification
- [ ] 定时器兜底：tick 检测时间跳跃
- [ ] `check_and_reset`：Paused 直接 return；delta > 阈值则重置 + 通知
- [ ] 启动时执行一次判定
- **验收**：时间跳跃>阈值且非暂停→重置；暂停跳跃→不重置；双平台唤醒触发。

### 阶段 9｜边缘场景与健壮性
- [ ] config/state 损坏回退
- [ ] 系统时间手动修改（前跳触发重置；后跳负 delta 按 0 处理不重置）
- [ ] 暂停/恢复幂等加锁
- [ ] 单实例锁
- [ ] 日志落盘跨平台路径，滚动
- [ ] **暂停状态重启后自动恢复 Running**（已确认）：启动读到 state.status==Paused 时，不保持暂停，而是从冻结的 remaining_seconds 自动恢复倒计时并发「已恢复工作阶段」通知

### 阶段 10｜测试、打包与发布
- [ ] 单元测试（mock 时间）+ 集成测试（加速倍率）
- [ ] 手工测试清单（双平台）：自启、唤醒重置、双屏遮罩+热插拔、通知按钮暂停、断电恢复、8h 挂机内存
- [ ] 打包：**Windows 出 .exe（NSIS）安装包**，含 WebView2 引导；**macOS 出 .dmg**（含 Universal Binary）
- [ ] 代码签名：Mac 需 Developer ID 签名 + 公证（否则 Gatekeeper 拦截）；Windows 可选 EV 证书
- [ ] 图标资源、README（双平台安装/管理员权限说明/已知限制）
- **验收**：干净 Win10/11 与 macOS 机器安装运行，核心功能正常。

---

## 五、阶段依赖与并行关系

```
0 脚手架 → 1 数据模型 → 2 计时引擎（核心）
                          ├─ 3 托盘   ─┐
                          ├─ 4 通知   ─┤ 可并行
                          └─ 5 遮罩UI ─┘
                               └─ 6 配置界面
                                    └─ 7 快捷键跳过
                                         └─ 8 过夜重置
                                              └─ 9 健壮性
                                                   └─ 10 测试打包
```

---

## 六、关键风险与对策

| 风险 | 影响 | 对策 |
|---|---|---|
| macOS 全屏窗口进入独立 Space，无法跨显示器同时覆盖 | 休息遮罩在 Mac 体验差 | 不用原生 fullscreen，用无边框置顶窗口覆盖屏幕区域；监听 Space 切换重新展示 |
| macOS 通知未授权 | 通知失效 | 首启请求授权；被拒降级 tooltip+配置红点 |
| Mac 代码签名/公证缺失 | Gatekeeper 拦截无法运行 | 需 Developer ID + notarytool；无证书则文档说明用户需手动放行 |
| 多显示器遮罩置顶失效 | 休息提醒失效 | 定时 re-raise；失焦监听；阶段5优先攻关 |
| Tauri 通知按钮回调机制 | 暂停按钮不生效 | 早期 spike；备选自定义小窗口按钮 |
| 系统挂起计时失准 | 倒计时不准 | 强制系统时间戳推进 |
| WebView2 在 Win10 未装 | 无法启动 | 安装包捆绑 WebView2 引导 |
| 快捷键被占用 | 跳过失效 | 注册失败检测+提示；预留可配置 |
| 跨平台维护成本翻倍 | 进度风险 | 平台抽象 trait 收敛差异；先 Win 后 Mac 里程碑推进 |

---

## 七、里程碑

| 里程碑 | 阶段 | 产出 | 平台 |
|---|---|---|---|
| M1 可用原型 | 0→2+3+4 | 循环计时+托盘+通知（休息仅通知） | Windows 先行 |
| M2 强休闭环 | +5 | 全屏遮罩+双屏+热插拔 | Windows；Mac 适配 |
| M3 可配置 | +6 | 配置面板 | 双平台 |
| M4 跳过与重置 | +7+8 | 快捷键跳过+过夜重置 | 双平台 |
| M5 健壮发布 | +9+10 | 异常恢复+签名打包 | 双平台 .exe/.dmg |

**建议**：先在 Windows 打通 M1–M2，验证最高风险的遮罩方案，再做 macOS 适配。避免双平台同步踩坑。

---

## 八、交互流程细化

### 8.1 首次启动引导
1. 读取 config（无则生成默认）→ 无 state → 初始化阶段 0。
2. **macOS**：请求通知授权（弹系统对话框）。
3. 托盘显示「工作中」，发「开始工作阶段」通知（Mac 若未授权则仅 tooltip）。
4. 首次可在配置界面开启「开机自启」（Win 可选管理员自启）。

### 8.2 工作阶段
- 每秒更新 remaining；每分钟写 state。
- 剩余 N 分钟（config）→ 发「即将进入休息，请保存工作」通知 +「暂停循环」按钮。
- 用户：无视 → 归零进休息；点暂停 → 冻结。

### 8.3 进入休息
- fullscreen：所有显示器遮罩；主屏大字倒计时 + 小字「按 Ctrl/Cmd+Shift+F2 跳过休息」。
- popup：右下角小窗。
- 状态 → Resting。

### 8.4 休息中跳过 / 休息结束
- **跳过**（快捷键，可配置，默认 Ctrl/Cmd+Shift+F2）：关遮罩 → **跳到下一个 work 阶段** → 发工作开始通知。
- **自然结束**：关遮罩 → 推进下一阶段（index+1）→ 若 work 发通知；若连续 rest 再触发遮罩。
- 注意：跳过是「跳到下一个 work」，自然结束是「推进一格」，二者推进逻辑不同。

### 8.5 配置修改与生效
- 配置界面保存 → 写 config.json → 弹提示「已保存，需重置计时或重启生效」。
- 用户通过托盘「重置计时」→ 引擎重新读 config → 从阶段 0 开始 → 发「已重置」通知。
- 或用户退出重启应用 → 启动时读新 config。

### 8.6 暂停与恢复
- 托盘/通知按钮暂停 → 冻结 remaining，托盘变灰。
- 恢复 → 从冻结值继续。
- **暂停期不触发过夜重置**。
- **重启行为**（已确认）：若退出时处于 Paused，下次启动读到 Paused 状态后**自动恢复 Running**，从冻结的 remaining 继续倒计时，并发「已恢复工作阶段」通知（不保持暂停）。

### 8.7 长时间离开/过夜
- 唤醒或时间跳跃 → check_and_reset → 非暂停且 delta>阈值 → 重置阶段0 + 通知。

---

## 九、确认结论与发布前遗留事项

### 9.1 已确认结论（第二轮）
1. **前端框架**：Vue 3 + TS（默认）。
2. **配置生效策略**：保存只写文件，不自动生效；需托盘「重置计时」或重启应用才生效，UI 保存时提示。
3. **跳过休息**：提供全局快捷键，**v1 即支持配置界面自定义**（默认 Ctrl+Shift+F2 / Cmd+Shift+F2）；跳过后**进入下一个 work 阶段**。
4. **管理员自启**：Windows 提供可选开关；macOS 无此概念。
5. **目标平台**：Windows（.exe/NSIS）+ macOS（.dmg，Universal Binary）；**Linux 不纳入 v1**，架构预留。
6. **macOS 遮罩**：无边框置顶窗口覆盖屏幕区域，不用原生 fullscreen；监听 Space 切换重新展示。
7. **暂停重启**：读到 Paused 状态自动恢复 Running，从冻结 remaining 继续。
8. **自动更新**：v1 不做，后续可接 Tauri updater。

### 9.2 发布前需落实（不影响编码启动）
- **macOS 代码签名**：是否有 Apple Developer ID 账号？
  - 有 → notarytool 公证，用户双击即可运行。
  - 无 → 文档说明用户需「右键 → 打开」首次放行；或仅自用分发。
- **Windows 代码签名**：是否购买 EV/OV 证书？无则 SmartScreen 会拦截首次运行。
- **图标/品牌**：托盘三状态图标、应用图标、dmg 背景图设计稿。
- **版本号**：v1.0.0 起步。

### 9.3 配置文件扩展（含快捷键）
config.json 在原需求基础上新增字段：
```json
{
  "stages": [ { "type": "work", "minutes": 45 } ],
  "rest_reminder_mode": "fullscreen",
  "reset_threshold_minutes": 30,
  "notify_before_work_end_minutes": 1,
  "skip_shortcut": "CmdOrCtrl+Shift+F2",
  "run_as_admin_autostart": false
}
```
- `skip_shortcut`：Tauri accelerator 格式，跨平台 `CmdOrCtrl` 自动映射 Ctrl/Cmd。
- `run_as_admin_autostart`：仅 Windows 生效，macOS 忽略。
