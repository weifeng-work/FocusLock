# FocusLock

> 自定义循环、到点即强休的跨平台专注节奏助手

FocusLock 是一款轻量级专注节奏管理工具。通过自定义工作/休息阶段循环，到点以全屏遮罩或弹窗强休，帮你守住节奏。

## 特性

- **自定义阶段循环**：自由编排工作/休息阶段及时长（如 45 分钟工作 + 15 分钟休息）
- **到点强休**：休息时全屏遮罩覆盖所有显示器，软强制让你停下来
- **多显示器支持**：主屏大字倒计时，副屏黑罩，热插拔自动补罩
- **系统托盘常驻**：三状态图标（工作中蓝/休息中绿/已暂停灰），悬停实时倒计时
- **全局快捷键**：默认 `CmdOrCtrl+Shift+F2` 跳过当前休息，可在配置中自定义
- **系统通知**：工作开始、即将休息、休息开始、跳过、重置等事件通知
- **断电恢复**：基于系统时间戳推进，挂起/重启不丢计时；暂停重启自动恢复
- **过夜重置**：长时间离开自动重置到第一阶段（暂停状态不触发）
- **跨平台**：Windows + macOS（Linux 暂未纳入）

## 技术栈

- **Tauri 2.x**（Rust 后端 + WebView 前端）
- **Vue 3 + TypeScript + Vite**
- **Pinia** 状态管理 / **Vue Router** 路由
- **tauri-plugin** 系列：single-instance / global-shortcut / notification / autostart / dialog

## 项目结构

```
大番茄/
├── src/                          # 前端源码
│   ├── views/
│   │   ├── Settings.vue          # 配置面板
│   │   └── Overlay.vue           # 休息遮罩（主屏/副屏/popup 三态）
│   ├── App.vue
│   ├── main.ts
│   └── types.ts
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── lib.rs                # 应用入口（setup、事件桥接、命令注册）
│   │   ├── main.rs
│   │   ├── config.rs             # 配置数据模型与持久化
│   │   ├── state.rs              # 运行状态持久化
│   │   ├── engine/
│   │   │   ├── timer.rs          # 计时引擎核心（tokio tick + 状态机）
│   │   │   ├── stage.rs          # 阶段循环游标（纯逻辑）
│   │   │   └── reset.rs          # 过夜重置判定
│   │   ├── platform/
│   │   │   ├── mod.rs            # 平台抽象 trait
│   │   │   ├── windows.rs        # Windows 显示器枚举
│   │   │   └── macos.rs          # macOS NSScreen（移植时填充）
│   │   ├── tray.rs               # 系统托盘
│   │   ├── tray_icons.rs         # 三状态图标 RGBA（自动生成）
│   │   ├── notify.rs             # 系统通知
│   │   ├── overlay.rs            # 遮罩窗口管理器
│   │   ├── shortcut.rs           # 全局快捷键
│   │   └── commands.rs           # Tauri command（前端 invoke 入口）
│   ├── icons/
│   │   └── tray/                 # 三状态托盘图标源
│   ├── Cargo.toml
│   └── tauri.conf.json
├── scripts/
│   ├── gen_icon.py               # 应用图标源图
│   ├── gen_tray_icons.py         # 三状态托盘图标
│   └── gen_tray_rgba.py          # 托盘图标转 Rust RGBA 源
├── IMPLEMENTATION_PLAN.md        # 实施计划 v2.1
└── package.json
```

## 开发

### 环境要求

- [Rust](https://www.rust-lang.org/) 1.77+
- [Node.js](https://nodejs.org/) 18+
- [Tauri 2 CLI 前置依赖](https://v2.tauri.app/start/prerequisites/)

### 运行

```bash
# 安装前端依赖
npm install

# 开发模式（启动前端 dev server + Tauri 开发窗口）
npm run tauri:dev

# 构建安装包（Windows 产出 NSIS .exe）
npm run tauri:build
```

开发模式下前端 dev server 端口为 `4317`（避开 Tauri 默认 1420，便于多 Tauri 项目并行）。

### 测试

```bash
cd src-tauri
cargo test --lib
```

覆盖配置校验、状态序列化、阶段循环、跳过休息、暂停恢复、重置重载、过夜重置判定等核心逻辑。

## 配置

配置文件位于：

- Windows: `%APPDATA%/FocusLock/config.json`
- macOS: `~/Library/Application Support/FocusLock/config.json`

首次启动自动生成默认配置（工作 45 分钟 / 休息 15 分钟 / 全屏遮罩）。

配置修改后**不会自动生效**，需通过托盘菜单「重置计时」或重启应用（快捷键即时生效除外）。这是有意设计：避免运行中切换节奏造成混乱。

## 使用

1. 安装后启动，应用常驻系统托盘后台运行
2. 默认开始第一个工作阶段倒计时
3. 托盘图标颜色反映状态：蓝色工作中 / 绿色休息中 / 灰色已暂停
4. 悬停托盘查看剩余时间
5. 右键托盘：暂停 / 恢复 / 重置计时 / 打开配置 / 退出
6. 左键托盘：打开配置面板
7. 工作结束前 1 分钟收到通知
8. 休息时全屏遮罩出现，按 `Ctrl+Shift+F2`（Mac 为 `Cmd+Shift+F2`）跳过

## 状态机

```
        进入 work 阶段              work 归零
Running ──────────────► (倒计时) ──────────► 推进到下一阶段
   ▲                                          │
   │ 恢复                                     │ 若下一阶段是 rest
   │                                          ▼
Paused ◄────── 暂停 ────── Resting ◄────── 进入 rest 阶段
                                  │
                                  │ rest 归零 / 跳过
                                  ▼
                              推进到下一 work
```

- 暂停可从 Running 或 Resting 进入，冻结倒计时
- 暂停期间不触发过夜重置
- 暂停状态重启后自动恢复 Running
- 跳过休息 → 向后扫描找下一个 work 阶段（非简单推进一格）

## 下载

前往 [Releases](../../releases) 下载最新安装包：

- Windows: `FocusLock_x.x.x_x64-setup.exe`（NSIS 安装包，内置 WebView2 引导）

## 路线图

- [x] 阶段 0-7：脚手架、数据模型、计时引擎、托盘、通知、遮罩、配置、快捷键
- [x] 阶段 10：Windows 打包
- [ ] macOS 适配（NSScreen 实现、menu bar、通知授权）
- [ ] 平台电源事件原生监听（Win WM_POWERBROADCAST / Mac NSWorkspaceDidWakeNotification）
- [ ] 边缘场景健壮性增强
- [ ] 自动更新

## 许可证

[MIT](LICENSE)
