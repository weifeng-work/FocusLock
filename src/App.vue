<script setup lang="ts">
// 根组件：承载 router-view，监听全局引擎事件
// 1. 播放音效（work_end / rest_end）
// 2. 处理时段结束动作（PeriodEndAction）：popup / fullscreen / black_screen
import { onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { handlePlaySoundEvent } from "./utils/sound";
import { initLocale } from "./locales";
import PeriodEndOverlay from "./views/PeriodEndOverlay.vue";
import type { PeriodEndAction } from "./types";

let unlisten: UnlistenFn | null = null;
const periodAction = ref<PeriodEndAction | null>(null);

onMounted(async () => {
  await initLocale();

  // 监听引擎事件
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
        // 后端发来 { action: PeriodEndAction, ... }
        const action = payload.action;
        if (action) {
          periodAction.value = action;
        }
      }
    }
  );
});

function closePeriodOverlay() {
  periodAction.value = null;
}

onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <router-view />
  <PeriodEndOverlay :action="periodAction" @close="closePeriodOverlay" />
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
