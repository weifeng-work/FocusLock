<script setup lang="ts">
// 根组件：承载 router-view，监听全局引擎事件并播放音效
import { onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { handlePlaySoundEvent } from "./utils/sound";
import { initLocale } from "./locales";

let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  await initLocale();

  // 监听引擎事件，播放音效 + 处理时段结束动作
  unlisten = await listen<{ type: string; [key: string]: any }>(
    "engine-event",
    (event) => {
      const payload = event.payload;
      if (payload.type === "play_sound") {
        const soundType = payload.sound_type;
        if (soundType) {
          handlePlaySoundEvent(soundType);
        }
      } else if (payload.type === "period_ended_action") {
        handlePeriodEndAction(payload.action);
      }
    }
  );
});

function handlePeriodEndAction(action: any) {
  if (!action) return;
  if (action.type === "none") return;
  // 基础实现：用 alert 显示时段结束提示
  if (action.type === "popup" || action.type === "fullscreen" || action.type === "black_screen") {
    const text = action.text || "时段结束";
    alert(`FocusLock\n${text}`);
    // 播放提示音
    if (action.sound) {
      const sound = action.sound;
      if (sound !== "none") {
        const soundType = sound === "builtin" ? "builtin" : `custom:${sound.custom}`;
        handlePlaySoundEvent(soundType);
      }
    }
  }
}

onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <router-view />
</template>

<style>
html,
body,
#app {
  margin: 0;
  padding: 0;
  height: 100%;
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
}
</style>
