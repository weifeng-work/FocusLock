import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHashHistory } from "vue-router";
import App from "./App.vue";
import Settings from "./views/Settings.vue";
import Overlay from "./views/Overlay.vue";

// 用 hash history：遮罩窗口 URL 形如 /#/overlay?primary=1 由后端构造
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/settings" },
    { path: "/settings", component: Settings },
    { path: "/overlay", component: Overlay },
  ],
});

createApp(App).use(createPinia()).use(router).mount("#app");
