// i18n 国际化工具
// 轻量级实现，不依赖 vue-i18n

import { ref, watch } from "vue";
import { zh } from "./zh";
import { en } from "./en";

export type Locale = "zh" | "en";
export type Messages = typeof zh;

const messages: Record<Locale, Messages> = { zh, en };

// 当前语言（响应式）
export const currentLocale = ref<Locale>("zh");

// 获取翻译
export function t(key: string): string {
  const locale = currentLocale.value;
  const msg = messages[locale] || messages.zh;

  // 支持嵌套键，如 "settings.title"
  const keys = key.split(".");
  let result: any = msg;
  for (const k of keys) {
    if (result && typeof result === "object" && k in result) {
      result = result[k];
    } else {
      return key; // 键不存在，返回原键
    }
  }
  return typeof result === "string" ? result : key;
}

// 切换语言
export function setLocale(locale: Locale) {
  currentLocale.value = locale;
  // 保存到 localStorage
  localStorage.setItem("focuslock-language", locale);
  // 保存到配置文件（通过后端）
  saveLanguageSetting(locale);
}

// 从 localStorage 读取语言设置
export function loadLocale(): Locale {
  const saved = localStorage.getItem("focuslock-language");
  if (saved === "en" || saved === "zh") {
    currentLocale.value = saved;
  }
  return currentLocale.value;
}

// 保存语言设置到配置文件
async function saveLanguageSetting(locale: Locale) {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    // 获取当前配置，修改 language，然后保存
    const config = await invoke<any>("get_config");
    config.language = locale;
    await invoke("save_config", { config, app: null });
  } catch (e) {
    console.warn("保存语言设置失败:", e);
  }
}

// 初始化语言设置
export async function initLocale(): Promise<Locale> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const config = await invoke<any>("get_config");
    if (config.language === "en" || config.language === "zh") {
      currentLocale.value = config.language;
      localStorage.setItem("focuslock-language", config.language);
    }
  } catch (e) {
    console.warn("读取配置语言失败:", e);
  }
  // 如果配置中没有，从 localStorage 读取
  loadLocale();
  return currentLocale.value;
}

// Vue 组合式函数
export function useI18n() {
  return {
    currentLocale,
    setLocale,
    t,
  };
}
