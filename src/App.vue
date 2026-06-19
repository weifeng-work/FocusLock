<script setup lang="ts">
// 根组件：仅承载 router-view，无全局 UI
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
