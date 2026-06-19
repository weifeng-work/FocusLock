// 平台抽象层
//
// 所有平台相关能力（多显示器枚举、电源事件、自启、数据目录）收敛到此模块的 trait，
// 业务逻辑（计时引擎等）只依赖 trait，不直接调用平台 API。
// 各平台后端在 windows/ 与 macos/ 子模块中通过 #[cfg(...)] 选择性编译。
//
// 阶段 0：仅占位定义，后续阶段 5/8 实现。

pub mod macos;
pub mod windows;

/// 单个显示器的矩形区域（逻辑坐标）
#[derive(Debug, Clone)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub is_primary: bool,
}

/// 多显示器枚举抽象
pub trait MonitorApi: Send + Sync {
    /// 返回当前所有显示器的矩形信息
    fn list_monitors(&self) -> Vec<MonitorRect>;
}

/// 平台能力集合，由各平台后端构造后注入引擎
pub struct PlatformCapabilities {
    pub monitor: Box<dyn MonitorApi>,
}

/// 根据编译目标返回对应平台的能力集合
pub fn create_capabilities() -> PlatformCapabilities {
    #[cfg(windows)]
    {
        PlatformCapabilities {
            monitor: Box::new(windows::WindowsMonitorApi),
        }
    }
    #[cfg(target_os = "macos")]
    {
        PlatformCapabilities {
            monitor: Box::new(macos::MacosMonitorApi),
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    compile_error!("FocusLock 当前仅支持 Windows 与 macOS");
}
