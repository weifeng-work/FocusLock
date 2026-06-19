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

import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRoute } from "vue-router";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

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

// 平台相关快捷键提示文案
const isMac = navigator.platform.toUpperCase().includes("MAC");
const shortcutHint = isMac
  ? "按 Cmd+Shift+F2 跳过休息"
  : "按 Ctrl+Shift+F2 跳过休息";

let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  // 订阅后端每秒倒计时推送
  unlisten = await listen<number>("overlay-tick", (e) => {
    remaining.value = e.payload;
  });
});

onUnmounted(() => {
  unlisten?.();
});

// 软强制：拦截点击（除了「跳过休息」按钮）
function onSkip() {
  invoke("skip_rest");
}
</script>

<template>
  <!-- 副显示器：纯黑半透明，无任何 UI -->
  <div v-if="isSecondary" class="overlay-secondary" @click.prevent.stop>
    <!-- 故意空：拦截点击 -->
  </div>

  <!-- 主显示器：大字倒计时 -->
  <div
    v-else-if="isPrimary"
    class="overlay-primary"
    :class="{ 'is-popup': isPopup }"
    @click.prevent.stop
  >
    <div class="hint-top">休息中</div>
    <div class="countdown">{{ display }}</div>
    <div class="hint-bottom">{{ shortcutHint }}</div>
    <button v-if="isPopup" class="skip-btn" @click.stop="onSkip">跳过休息</button>
  </div>

  <!-- popup 模式：紧凑 -->
  <div v-else class="overlay-popup" @click.prevent.stop>
    <div class="popup-title">FocusLock 休息中</div>
    <div class="popup-countdown">{{ display }}</div>
    <button class="skip-btn" @click.stop="onSkip">跳过休息</button>
  </div>
</template>

<style scoped>
/* 副显示器：黑 90% 透明 */
.overlay-secondary {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.9);
  cursor: none;
  user-select: none;
}

/* 主显示器：黑 90% + 居中大字 */
.overlay-primary {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.9);
  color: #ffffff;
  user-select: none;
  cursor: none;
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

/* popup 模式：右下角紧凑卡片 */
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
