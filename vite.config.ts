import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// FocusLock 前端 dev server 配置
// 端口 4317：刻意避开 Tauri 默认端口 1420，防止与本机其他 Tauri 项目（npm run tauri dev）冲突
const FRONTEND_PORT = 4317;

export default defineConfig({
  plugins: [vue()],
  // Tauri 期望前端 dev server 在固定端口提供内容
  clearScreen: false,
  server: {
    port: FRONTEND_PORT,
    strictPort: true,
    // 监听 Tauri IPC
    host: "127.0.0.1",
  },
  // Tauri 构建产物期望在 dist/
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
});
