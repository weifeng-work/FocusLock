// English language pack
export const en = {
  // App name
  appName: "FocusLock",

  // Tray menu
  tray: {
    pause: "Pause",
    resume: "Resume",
    reset: "Reset Timer",
    settings: "Settings",
    exit: "Exit",
    language: "Language",
    languageZh: "中文",
    languageEn: "English",
  },

  // Settings panel
  settings: {
    title: "FocusLock Settings",
    stages: "Stage Cycle",
    stageWork: "Work",
    stageRest: "Rest",
    minutes: "min",
    addStage: "Add Stage",
    removeStage: "Remove",
    moveUp: "Move Up",
    moveDown: "Move Down",
    stageType: "Stage Type",
    duration: "Duration",

    restReminder: "Rest Reminder",
    modeFullscreen: "Fullscreen Overlay",
    modePopup: "Bottom-right Popup",

    overlayStyle: "Overlay Style",
    styleSemiTransparent: "Semi-transparent (Default)",
    styleFullBlack: "Full Black",
    styleDark: "Dark",

    restMessage: "Rest Message",
    restMessagePlaceholder: "Enter message to display during rest",

    sound: "Sound Effects",
    workEndSound: "Work End Sound",
    restEndSound: "Rest End Sound",
    soundNone: "None",
    soundBuiltin: "Built-in Beep",
    soundCustom: "Custom Audio",
    selectSoundFile: "Select Audio File",
    soundFileHint: "Supports mp3/wav/aac/ogg/flac/m4a formats",
    uploadedSounds: "Uploaded Audio Files",
    deleteSound: "Delete",
    deleteSoundConfirm: "Are you sure to delete '{name}'?",

    system: "System Settings",
    resetThreshold: "Away/Reset Threshold (minutes)",
    notifyBefore: "Notify Before Work Ends (minutes)",
    skipShortcut: "Skip Rest Shortcut",
    runAsAdmin: "Run as Admin on Startup (Windows only)",

    actions: "Actions",
    save: "Save Config",
    resetTimer: "Reset Timer",
    checkUpdate: "Check for Updates",

    about: "About",
    version: "Version",
    updateAvailable: "Update Available",
    upToDate: "Up to Date",
    githubRelease: "GitHub Releases",

    wechat_group: "WeChat Support Group",
    scanQrCode: "Scan to join FocusLock WeChat group",

    saveSuccess: "Config saved",
    saveError: "Save failed",

    // Schedule (added in v0.1.3)
    tabs: {
      general: "General",
      scheme: "Schemes",
      routine: "Routines",
      weekly: "Weekly",
      system: "System",
    },
    scheme: {
      title: "Scheme Management",
      hint: "A scheme defines a work/rest cycle. You can assign different schemes to different time periods.",
      newScheme: "New Scheme",
      cloneScheme: "Clone Current",
      deleteScheme: "Delete",
      deleteSchemeConfirm: "Delete scheme \"{name}\"? Schemes referenced by routines cannot be deleted.",
      builtIn: "Built-in",
      custom: "Custom",
      builtInWarnDelete: "Built-in schemes cannot be deleted (clone first)",
      builtInWarnEditName: "Built-in schemes cannot be renamed (clone first)",
    },
    routine: {
      title: "Routines",
      hint: "A routine assigns schemes to time periods. E.g. 8:00-12:00 uses Scheme A, 14:00-18:00 uses Scheme B.",
      newRoutine: "New Routine",
      deleteRoutine: "Delete",
      deleteRoutineConfirm: "Delete routine \"{name}\"?",
      newPeriod: "Add Period",
      deletePeriod: "Delete Period",
      periodStart: "Start",
      periodEnd: "End",
      periodScheme: "Use Scheme",
      periodEndAction: "End Action",
      selectScheme: "Select scheme",
    },
    weekly: {
      title: "Weekly Schedule",
      hint: "Assign a routine for each day of the week. Click a cell to select.",
      mon: "Mon",
      tue: "Tue",
      wed: "Wed",
      thu: "Thu",
      fri: "Fri",
      sat: "Sat",
      sun: "Sun",
      sameAsWeekday: "Same as weekday",
    },
    endAction: {
      none: "No Action",
      popup: "Popup Notice",
      fullscreen: "Fullscreen Overlay",
      black_screen: "Full Black Screen",
      text: "Message Text",
      textHint: "Leave empty for default",
      sound: "Sound",
      style: "Overlay Style",
    },
  },

  // Overlay
  overlay: {
    title: "FocusLock",
    restMessage: "Time to Rest",
    skip: "Skip Rest",
    pause: "Pause",
    resume: "Resume",
    timeRemaining: "Time Remaining",
  },

  // Notifications
  notification: {
    workEnd: "Work time is up, time to rest!",
    restEnd: "Rest is over, back to work!",
    prepareRest: "Rest time is coming",
  },

  // First run
  firstRun: {
    title: "Select Language / 选择语言",
    welcome: "Welcome to FocusLock",
    welcomeEn: "Welcome to FocusLock",
    selectLanguage: "Please select your language / 请选择您的语言",
    start: "Get Started / 开始使用",
  },
};
