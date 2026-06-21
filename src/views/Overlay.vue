<script setup lang="ts">
// 休息遮罩页
//
// URL 查询参数（后端构造）：
//   primary=1   主显示器：大字倒计时 + 文案 + 跳过按钮
//   primary=0   副显示器：纯黑半透明（不显示按钮，避免误触）
//   popup=1     popup 模式：紧凑卡片
//   remaining=N 初始剩余秒数
//   opacity=0-100  遮罩不透明度（默认 95）
//   msg=...     休息文案（URL-encoded）
//
// 倒计时：前端通过 setInterval 本地驱动，同时监听后端 "overlay-tick" 作为备用同步。
//
// 鼠标策略：恢复默认指针（不再 cursor: none），让用户能用鼠标操作。
// 解锁按钮：hover 后显示 2 秒倒计时，停留满 2 秒才真正解锁（防误触）。
//           同时显示键盘快捷键提示。
//
// 透明度：读取 opacity 参数，用 CSS 变量驱动。

import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRoute } from "vue-router";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const route = useRoute();

const isPrimary = computed(() => route.query.primary === "1");
const isPopup = computed(() => route.query.popup === "1");
const isSecondary = computed(() => !isPopup.value && !isPrimary.value);

const remaining = ref<number>(Number(route.query.remaining ?? 0));

// URL 参数：opacity (0-100) + msg（URL-encoded）
const overlayOpacity = computed(() => {
  const v = Number(route.query.opacity ?? 95);
  return Math.max(0, Math.min(100, isNaN(v) ? 95 : v));
});

const messageFromUrl = computed(() => {
  const raw = String(route.query.msg ?? "");
  if (!raw) return "";
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
});

const display = computed(() => {
  const m = Math.floor(remaining.value / 60);
  const s = remaining.value % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
});

// 平台相关快捷键提示文案
const isMac = navigator.platform.toUpperCase().includes("MAC");
const shortcutHint = isMac
  ? "按 Cmd+Shift+F2 跳过休息"
  : "按 Ctrl+Shift+F2 跳过休息";

// 鼠标悬停解锁：hover 后 2 秒倒计时
const hovering = ref(false);
const hoverCountdown = ref(0);
let hoverTimer: number | null = null;

function startHoverUnlock() {
  if (hovering.value) return;
  hovering.value = true;
  hoverCountdown.value = 2;
  hoverTimer = window.setInterval(() => {
    hoverCountdown.value -= 1;
    if (hoverCountdown.value <= 0) {
      if (hoverTimer) {
        clearInterval(hoverTimer);
        hoverTimer = null;
      }
      invoke("skip_rest").catch(() => {});
    }
  }, 1000);
}

function cancelHoverUnlock() {
  if (hoverTimer) {
    clearInterval(hoverTimer);
    hoverTimer = null;
  }
  hovering.value = false;
  hoverCountdown.value = 0;
}

function onSkipClick() {
  invoke("skip_rest").catch(() => {});
}

let unlisten: UnlistenFn | null = null;
let countdownTimer: number | null = null;

onMounted(async () => {
  // 前端本地驱动倒计时：每秒递减 remaining
  countdownTimer = window.setInterval(() => {
    if (remaining.value > 0) {
      remaining.value--;
    }
  }, 1000);

  // 同时订阅后端每秒倒计时推送（作为备用同步机制）
  unlisten = await listen<number>("overlay-tick", (e) => {
    remaining.value = e.payload;
  });
});

onUnmounted(() => {
  unlisten?.();
  if (hoverTimer) clearInterval(hoverTimer);
  if (countdownTimer) clearInterval(countdownTimer);
});

// 遮罩样式：使用 CSS 变量 --bg-alpha 控制透明度
const overlayStyle = computed(() => ({
  "--bg-alpha": (overlayOpacity.value / 100).toFixed(2),
}));
</script>

<template>
  <!-- 副显示器：根据透明度渲染背景 + "休息中" 小提示，避免全空让用户以为死机 -->
  <div
    v-if="isSecondary"
    class="overlay-secondary"
    :style="overlayStyle"
    @click.prevent.stop
  >
    <div class="secondary-hint">休息中</div>
  </div>

  <!-- 主显示器：大字倒计时 + 文案 + 解锁按钮 -->
  <div
    v-else-if="isPrimary"
    class="overlay-primary"
    :style="overlayStyle"
    @click.prevent.stop
  >
    <div class="hint-top">{{ messageFromUrl || "休息一下" }}</div>
    <div class="countdown">{{ display }}</div>

    <!-- 跳过按钮：hover 后 2 秒倒计时，停留满才解锁 -->
    <div
      class="unlock-area"
      @mouseenter="startHoverUnlock"
      @mouseleave="cancelHoverUnlock"
      @click.stop
    >
      <button class="unlock-btn" :class="{ 'is-hovering': hovering }">
        <span v-if="!hovering">将鼠标移到这里跳过休息</span>
        <span v-else>再停留 {{ hoverCountdown }} 秒解锁…</span>
      </button>
      <div class="shortcut-hint">{{ shortcutHint }}</div>
    </div>
  </div>

  <!-- popup 模式：紧凑卡片，保留原样 -->
  <div
    v-else
    class="overlay-popup"
    :style="overlayStyle"
    @click.prevent.stop
  >
    <div class="popup-title">FocusLock</div>
    <div class="popup-message">{{ messageFromUrl || "休息一下" }}</div>
    <div class="popup-countdown">{{ display }}</div>
    <button class="skip-btn" @click.stop="onSkipClick">跳过休息</button>
  </div>
</template>

<style scoped>
/* ========== 通用：CSS 变量驱动的遮罩背景 ========== */
.overlay-primary,
.overlay-secondary {
  background: rgba(0, 0, 0, var(--bg-alpha, 0.95));
  /* 恢复默认鼠标指针（不再 cursor: none） */
}

.overlay-popup {
  background: #1f1f1f;
}

/* ========== 副显示器基础样式 ========== */
.overlay-secondary {
  position: fixed;
  inset: 0;
  user-select: none;
  transition: background 0.3s ease;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 8vh;
}

.secondary-hint {
  color: rgba(255, 255, 255, 0.25);
  font-size: 14px;
  letter-spacing: 4px;
  font-weight: 300;
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
  transition: background 0.3s ease;
}

.hint-top {
  font-size: 24px;
  font-weight: 400;
  letter-spacing: 8px;
  margin-bottom: 32px;
  opacity: 0.7;
  text-align: center;
  padding: 0 32px;
}

.countdown {
  font-size: 22vw;
  font-weight: 200;
  font-variant-numeric: tabular-nums;
  line-height: 1;
  letter-spacing: -0.02em;
}

/* ========== 解锁按钮（hover 2 秒） ========== */
.unlock-area {
  position: fixed;
  bottom: 8vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.unlock-btn {
  padding: 14px 36px;
  background: transparent;
  color: #ffffff;
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 999px;
  font-size: 14px;
  letter-spacing: 2px;
  cursor: pointer;
  transition: all 0.2s ease;
  font-family: inherit;
  min-width: 240px;
}

.unlock-btn:hover,
.unlock-btn.is-hovering {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.5);
}

.shortcut-hint {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.4);
  letter-spacing: 1px;
}

/* ========== popup 模式：右下角紧凑卡片 ========== */
.overlay-popup {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #ffffff;
  border-radius: 12px;
  user-select: none;
}

.popup-title {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 4px;
  opacity: 0.6;
  letter-spacing: 2px;
}

.popup-message {
  font-size: 13px;
  opacity: 0.5;
  margin-bottom: 16px;
  text-align: center;
  padding: 0 24px;
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
  font-family: inherit;
}

.skip-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.6);
}
</style>
