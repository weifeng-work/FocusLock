<script setup lang="ts">
// 休息遮罩页（阶段 5）
//
// URL 查询参数（后端构造）：
//   primary=1   主显示器：大字倒计时 + 快捷键提示
//   primary=0   副显示器：纯黑半透明
//   popup=1     popup 模式：紧凑倒计时
//   remaining=N 初始剩余秒数
//
// 倒计时通过 listen("overlay-tick") 接收后端每秒推送。
// 拦截鼠标点击（pointer-events:none + 容器吞 click）实现软强制。
// 遮罩样式和提示词从配置读取（overlay_style / rest_message）

import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRoute } from "vue-router";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { Config, OverlayStyle } from "../types";

const route = useRoute();

const isPrimary = computed(() => route.query.primary === "1");
const isPopup = computed(() => route.query.popup === "1");
const isSecondary = computed(() => !isPopup.value && !isPrimary.value);

const remaining = ref<number>(Number(route.query.remaining ?? 0));

const display = computed(() => {
  const m = Math.floor(remaining.value / 60);
  const s = remaining.value % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
});

// 从配置加载的：遮罩样式 + 自定义提示词
const overlayStyle = ref<OverlayStyle>("semi_transparent");
const restMessage = ref("现在休息");

// 平台相关快捷键提示文案
const isMac = navigator.platform.toUpperCase().includes("MAC");
const shortcutHint = isMac
  ? "按 Cmd+Shift+F2 跳过休息"
  : "按 Ctrl+Shift+F2 跳过休息";

let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  // 加载配置获取遮罩样式和提示词
  try {
    const cfg = await invoke<Config>("get_config");
    overlayStyle.value = cfg.overlay_style;
    restMessage.value = cfg.rest_message || "现在休息";
  } catch {
    // 配置读取失败时使用默认值
  }

  // 订阅后端每秒倒计时推送
  unlisten = await listen<number>("overlay-tick", (e) => {
    remaining.value = e.payload;
  });
});

onUnmounted(() => {
  unlisten?.();
});

// 遮罩样式映射到 CSS 类名
const overlayClass = computed(() => {
  if (isSecondary.value) return `overlay-secondary style-${overlayStyle.value}`;
  if (isPrimary.value) return `overlay-primary style-${overlayStyle.value}`;
  return `overlay-popup`;
});

// 软强制：拦截点击（除了「跳过休息」按钮）
function onSkip() {
  invoke("skip_rest");
}
</script>

<template>
  <!-- 副显示器：根据样式渲染 -->
  <div v-if="isSecondary" :class="overlayClass" @click.prevent.stop>
    <!-- 故意空：拦截点击 -->
  </div>

  <!-- 主显示器：大字倒计时 + 自定义提示词 -->
  <div
    v-else-if="isPrimary"
    :class="overlayClass"
    @click.prevent.stop
  >
    <div class="hint-top">{{ restMessage }}</div>
    <div class="countdown">{{ display }}</div>
    <div class="hint-bottom">{{ shortcutHint }}</div>
    <button v-if="isPopup" class="skip-btn" @click.stop="onSkip">跳过休息</button>
  </div>

  <!-- popup 模式：紧凑 -->
  <div v-else class="overlay-popup" @click.prevent.stop>
    <div class="popup-title">FocusLock {{ restMessage }}</div>
    <div class="popup-countdown">{{ display }}</div>
    <button class="skip-btn" @click.stop="onSkip">跳过休息</button>
  </div>
</template>

<style scoped>
/* ========== 遮罩样式变体 ========== */

/* --- 半透明（默认，当前效果）--- */
.style-semi_transparent.overlay-secondary,
.style-semi_transparent.overlay-primary {
  background: rgba(0, 0, 0, 0.9);
}

/* --- 纯黑不透明 --- */
.style-full_black.overlay-secondary,
.style-full_black.overlay-primary {
  background: #000000;
}

/* --- 暗色调（深蓝灰）--- */
.style-dark.overlay-secondary,
.style-dark.overlay-primary {
  background: rgba(18, 18, 24, 0.95);
}

/* ========== 副显示器基础样式 ========== */
.overlay-secondary {
  position: fixed;
  inset: 0;
  cursor: none;
  user-select: none;
  transition: background 0.3s ease;
}

/* ========== 主显示器基础样式 ========== */
.overlay-primary {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #ffffff;
  user-select: none;
  cursor: none;
  transition: background 0.3s ease;
}

.hint-top {
  font-size: 24px;
  font-weight: 400;
  letter-spacing: 8px;
  margin-bottom: 32px;
  opacity: 0.7;
}

.countdown {
  font-size: 22vw;
  font-weight: 200;
  font-variant-numeric: tabular-nums;
  line-height: 1;
  letter-spacing: -0.02em;
}

.hint-bottom {
  position: fixed;
  bottom: 5vh;
  font-size: 16px;
  opacity: 0.5;
  cursor: pointer; /* 允许点击提示区域 */
}

.hint-bottom:hover {
  opacity: 0.9;
}

/* ========== popup 模式：右下角紧凑卡片 ========== */
.overlay-popup {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: #1f1f1f;
  color: #ffffff;
  border-radius: 12px;
  user-select: none;
}

.popup-title {
  font-size: 16px;
  font-weight: 500;
  margin-bottom: 16px;
  opacity: 0.8;
}

.popup-countdown {
  font-size: 64px;
  font-weight: 300;
  font-variant-numeric: tabular-nums;
  margin-bottom: 24px;
}

.skip-btn {
  padding: 8px 20px;
  background: transparent;
  color: #ffffff;
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.15s;
}

.skip-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.6);
}
</style>
