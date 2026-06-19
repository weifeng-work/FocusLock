// 中文语言包
export const zh = {
  // 应用名称
  appName: "FocusLock 专注锁",

  // 托盘菜单
  tray: {
    pause: "暂停",
    resume: "继续",
    reset: "重置计时",
    settings: "设置",
    exit: "退出",
    language: "语言",
    languageZh: "中文",
    languageEn: "English",
  },

  // 设置面板
  settings: {
    title: "FocusLock 设置",
    stages: "阶段循环",
    stageWork: "工作",
    stageRest: "休息",
    minutes: "分钟",
    addStage: "添加阶段",
    removeStage: "删除",
    moveUp: "上移",
    moveDown: "下移",
    stageType: "阶段类型",
    duration: "持续时间",

    restReminder: "休息提醒",
    modeFullscreen: "全屏遮罩",
    modePopup: "右下角弹窗",

    overlayStyle: "遮罩样式",
    styleSemiTransparent: "半透明（默认）",
    styleFullBlack: "全黑",
    styleDark: "暗色",

    restMessage: "休息提示词",
    restMessagePlaceholder: "输入休息时显示的提示文字",

    sound: "提示音",
    workEndSound: "工作结束提示音",
    restEndSound: "休息结束提示音",
    soundNone: "无声",
    soundBuiltin: "内置提示音",
    soundCustom: "自定义音频",
    selectSoundFile: "选择音频文件",
    soundFileHint: "支持 mp3/wav/aac/ogg/flac/m4a 格式",
    uploadedSounds: "已上传的音频",
    deleteSound: "删除",
    deleteSoundConfirm: "确定要删除「{name}」吗？",

    system: "系统设置",
    resetThreshold: "过夜/离开重置阈值（分钟）",
    notifyBefore: "工作结束前提醒（分钟）",
    skipShortcut: "跳过休息快捷键",
    runAsAdmin: "以管理员权限自启（仅 Windows）",

    actions: "操作",
    save: "保存配置",
    resetTimer: "重置计时",
    checkUpdate: "检查更新",

    about: "关于",
    version: "版本",
    updateAvailable: "发现新版本",
    upToDate: "已是最新版本",
    githubRelease: "GitHub 发布页",

    wechat_group: "微信客服群",
    scanQrCode: "扫码加入 FocusLock 微信客服群",

    saveSuccess: "配置已保存",
    saveError: "保存失败",

    // 作息表（v0.1.3 新增）
    tabs: {
      general: "通用设置",
      scheme: "方案管理",
      routine: "作息表",
      weekly: "周配置",
      system: "系统",
    },
    scheme: {
      title: "方案管理",
      hint: "方案定义一个工作/休息循环。可为不同时间段指定不同方案。",
      newScheme: "新建方案",
      cloneScheme: "复制当前方案",
      deleteScheme: "删除方案",
      deleteSchemeConfirm: "确定要删除方案「{name}」吗？被作息表引用的方案无法删除。",
      builtIn: "内置",
      custom: "自定义",
      builtInWarnDelete: "内置方案不可删除（可复制后修改）",
      builtInWarnEditName: "内置方案不可重命名（可复制后修改）",
    },
    routine: {
      title: "作息表",
      hint: "作息表是按时间段安排方案的容器。例如 8:00-12:00 用方案 A，14:00-18:00 用方案 B。",
      newRoutine: "新建作息表",
      deleteRoutine: "删除作息表",
      deleteRoutineConfirm: "确定要删除作息表「{name}」吗？",
      newPeriod: "添加时间段",
      deletePeriod: "删除时间段",
      periodStart: "开始",
      periodEnd: "结束",
      periodScheme: "使用方案",
      periodEndAction: "时段结束动作",
      selectScheme: "选择方案",
    },
    weekly: {
      title: "周配置",
      hint: "为每周的每一天指定一个作息表。点击单元格选择。",
      mon: "周一",
      tue: "周二",
      wed: "周三",
      thu: "周四",
      fri: "周五",
      sat: "周六",
      sun: "周日",
      sameAsWeekday: "同工作日",
    },
    endAction: {
      none: "无动作",
      popup: "弹窗提示",
      fullscreen: "全屏遮罩",
      black_screen: "全黑屏",
      text: "提示文字",
      textHint: "留空则使用默认提示",
      sound: "提示音",
      style: "遮罩样式",
    },
  },

  // 遮罩
  overlay: {
    title: "FocusLock",
    restMessage: "现在休息",
    skip: "跳过休息",
    pause: "暂停",
    resume: "继续",
    timeRemaining: "剩余时间",
  },

  // 通知
  notification: {
    workEnd: "工作时间结束，该休息了！",
    restEnd: "休息结束，返回工作！",
    prepareRest: "即将进入休息时间",
  },

  // 首次启动
  firstRun: {
    title: "选择语言 / Select Language",
    welcome: "欢迎使用 FocusLock",
    welcomeEn: "Welcome to FocusLock",
    selectLanguage: "请选择您的语言 / Please select your language",
    start: "开始使用",
  },
};
