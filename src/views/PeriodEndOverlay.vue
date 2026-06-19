<script setup lang="ts">
// 时段结束动作展示组件
// 根据 PeriodEndAction 的 type 显示对应效果：
// - none: 不渲染
// - popup: 居中弹窗（带遮罩但不强制全屏，可关闭）
// - fullscreen: 全屏遮罩 + 自定义文字/样式
// - black_screen: 全黑屏 + 自定义文字
// 关闭方式：弹窗可点 X 或 Esc；全屏/黑屏按 Esc 或点击屏幕中央区域
import { ref, computed, onMounted, onUnmounted, watch } from "vue";
import type { PeriodEndAction } from "../types";
import { handlePlaySoundEvent } from "../utils/sound";

const props = defineProps<{
  action: PeriodEndAction | null;
}>();

const emit = defineEmits<{
  (e: "close"): void;
}>();

// popup 模式用的倒计时（5 秒后自动关闭）
const popupCountdown = ref(5);
let popupTimer: number | null = null;

// 是否可见
const visible = computed(() => {
  if (!props.action) return false;
  return props.action.type !== "none";
});

const text = computed(() => {
  if (!props.action || props.action.type === "none") return "";
  return props.action.text || "时段结束";
});

const style = computed(() => {
  if (!props.action || props.action.type !== "fullscreen") return "semi_transparent";
  return props.action.style || "semi_transparent";
});

// 样式映射
const overlayClass = computed(() => {
  if (!props.action) return "";
  if (props.action.type === "black_screen") return "black-screen";
  if (props.action.type === "fullscreen") {
    return `fullscreen style-${style.value}`;
  }
  if (props.action.type === "popup") return "popup-mask";
  return "";
});

const isPopup = computed(() => props.action?.type === "popup");
const isFullscreen = computed(() =>
  props.action?.type === "fullscreen" || props.action?.type === "black_screen"
);

// 播放音效
function playActionSound() {
  if (!props.action) return;
  let sound: any = null;
  if (props.action.type === "popup" || props.action.type === "fullscreen" || props.action.type === "black_screen") {
    sound = props.action.sound;
  }
  if (!sound || sound === "none") return;
  const soundType = sound === "builtin" ? "builtin" : `custom:${sound.custom}`;
  handlePlaySoundEvent(soundType);
}

// 关闭
function close() {
  emit("close");
}

// Esc 键关闭
function onKeyDown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    close();
  }
}

// popup 模式 5 秒倒计时
function startPopupCountdown() {
  popupCountdown.value = 5;
  if (popupTimer !== null) {
    clearInterval(popupTimer);
  }
  popupTimer = window.setInterval(() => {
    popupCountdown.value -= 1;
    if (popupCountdown.value <= 0) {
      if (popupTimer !== null) {
        clearInterval(popupTimer);
        popupTimer = null;
      }
      close();
    }
  }, 1000);
}

watch(
  () => props.action,
  (newAction) => {
    if (newAction && newAction.type !== "none") {
      playActionSound();
      if (newAction.type === "popup") {
        startPopupCountdown();
      } else {
        // 全屏/黑屏模式清除可能存在的 popup 倒计时
        if (popupTimer !== null) {
          clearInterval(popupTimer);
          popupTimer = null;
        }
      }
    } else {
      if (popupTimer !== null) {
        clearInterval(popupTimer);
        popupTimer = null;
      }
    }
  },
  { immediate: true }
);

onMounted(() => {
  document.addEventListener("keydown", onKeyDown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeyDown);
  if (popupTimer !== null) {
    clearInterval(popupTimer);
  }
});
</script>

<template>
  <Teleport to="body">
    <Transition name="period-fade">
      <div
        v-if="visible"
        :class="['period-overlay', overlayClass]"
        @click.self="isFullscreen ? close() : null"
      >
        <!-- Popup 模式：居中卡片 -->
        <div v-if="isPopup" class="popup-card" @click.stop>
          <button class="popup-close" @click="close" title="关闭 (Esc)">×</button>
          <div class="popup-icon">⏰</div>
          <div class="popup-text">{{ text }}</div>
          <div class="popup-hint">{{ popupCountdown }} 秒后自动关闭</div>
        </div>

        <!-- Fullscreen / Black screen 模式：中央大文字 -->
        <div v-else-if="isFullscreen" class="fullscreen-content" @click.stop="close">
          <div class="fullscreen-text">{{ text }}</div>
          <div class="fullscreen-hint">点击屏幕或按 Esc 关闭</div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.period-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
}

/* Popup 模式：半透明遮罩 + 居中卡片 */
.popup-mask {
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(2px);
}

.popup-card {
  position: relative;
  min-width: 320px;
  max-width: 480px;
  padding: 32px 28px 24px;
  background: #ffffff;
  border-radius: 12px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
  text-align: center;
  cursor: default;
}

.popup-close {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  font-size: 20px;
  color: #888;
  cursor: pointer;
  border-radius: 4px;
}

.popup-close:hover {
  background: #f0f0f0;
  color: #333;
}

.popup-icon {
  font-size: 48px;
  margin-bottom: 12px;
}

.popup-text {
  font-size: 20px;
  font-weight: 500;
  color: #2c2c2a;
  line-height: 1.5;
  margin-bottom: 12px;
  word-break: break-word;
}

.popup-hint {
  font-size: 12px;
  color: #888;
}

/* Fullscreen 模式 */
.fullscreen {
  cursor: pointer;
}

.fullscreen.style-semi_transparent {
  background: rgba(0, 0, 0, 0.7);
  color: #ffffff;
}

.fullscreen.style-full_black {
  background: #000000;
  color: #ffffff;
}

.fullscreen.style-dark {
  background: #1a1a1a;
  color: #e0e0e0;
}

.black-screen {
  background: #000000;
  color: #ffffff;
  cursor: pointer;
}

.fullscreen-content {
  text-align: center;
  max-width: 80vw;
  cursor: pointer;
}

.fullscreen-text {
  font-size: 48px;
  font-weight: 500;
  line-height: 1.4;
  margin-bottom: 24px;
  word-break: break-word;
  text-shadow: 0 2px 12px rgba(0, 0, 0, 0.5);
}

.fullscreen-hint {
  font-size: 16px;
  opacity: 0.6;
  font-weight: 300;
}

/* 过渡动画 */
.period-fade-enter-active,
.period-fade-leave-active {
  transition: opacity 0.25s ease;
}

.period-fade-enter-from,
.period-fade-leave-to {
  opacity: 0;
}
</style>
