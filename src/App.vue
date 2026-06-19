<script setup lang="ts">
// 根组件：承载 router-view，监听全局引擎事件并播放音效
import { onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { handlePlaySoundEvent } from "./utils/sound";
import { initLocale } from "./locales";

let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  // 初始化语言设置
  await initLocale();

  // 监听引擎事件，播放音效
  unlisten = await listen<{ type: string; [key: string]: any }>(
    "engine-event",
    (event) => {
      const payload = event.payload;
      if (payload.type === "PlaySound") {
        const soundType = payload.sound_type;
        if (soundType) {
          handlePlaySoundEvent(soundType);
        }
      }
    }
  );
});

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
