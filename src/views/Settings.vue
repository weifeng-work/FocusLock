<script setup lang="ts">
// 配置面板（阶段 6）
// - 阶段列表编辑器：增删/上下移/改类型/改分钟
// - rest_reminder_mode 单选（fullscreen / popup）
// - overlay_style 选择：半透明 / 全黑 / 暗色
// - rest_message 自定义休息提示词
// - reset_threshold_minutes、notify_before_work_end_minutes 数字输入
// - skip_shortcut 文本输入（Tauri accelerator 格式）
// - Windows 专属：run_as_admin_autostart 开关
// - 检查更新功能
// - 微信客服群二维码
// - 保存 → invoke("save_config") → 提示需重置/重启生效

import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Config, Stage, StageType, RestReminderMode, OverlayStyle } from "../types";

const config = ref<Config | null>(null);
const saving = ref(false);
const message = ref<{ type: "ok" | "warn" | "err"; text: string } | null>(null);
const isMac = navigator.platform.toUpperCase().includes("MAC");

// 检查更新相关状态
const checkingUpdate = ref(false);
const updateResult = ref<{ type: "ok" | "warn" | "err" | "info"; text: string } | null>(null);

async function load() {
  config.value = await invoke<Config>("get_config");
}

onMounted(load);

function addStage(type: StageType) {
  config.value?.stages.push({ type, minutes: type === "work" ? 25 : 5 });
}

function removeStage(i: number) {
  config.value?.stages.splice(i, 1);
}

function moveUp(i: number) {
  if (i <= 0 || !config.value) return;
  const arr = config.value.stages;
  [arr[i - 1], arr[i]] = [arr[i], arr[i - 1]];
}

function moveDown(i: number) {
  if (!config.value || i >= config.value.stages.length - 1) return;
  const arr = config.value.stages;
  [arr[i + 1], arr[i]] = [arr[i], arr[i + 1]];
}

const hasWork = computed(() => config.value?.stages.some((s) => s.type === "work") ?? false);
const stagesValid = computed(() => (config.value?.stages.length ?? 0) > 0 && hasWork.value);

async function onSave() {
  if (!config.value || !stagesValid.value) {
    message.value = { type: "err", text: "配置无效：至少需要一个工作阶段" };
    return;
  }
  saving.value = true;
  message.value = null;
  try {
    const tip = await invoke<string>("save_config", { config: config.value });
    message.value = { type: "warn", text: tip };
  } catch (e) {
    message.value = { type: "err", text: String(e) };
  } finally {
    saving.value = false;
  }
}

async function onResetTimer() {
  await invoke("reset_timer");
  message.value = { type: "ok", text: "已重置计时，新配置立即生效" };
}

async function onCheckUpdate() {
  checkingUpdate.value = true;
  updateResult.value = null;
  try {
    const result = await invoke<{ current: string; latest: string; url: string; has_update: boolean }>("check_update");
    if (result.has_update) {
      updateResult.value = {
        type: "warn",
        text: `发现新版本 ${result.latest}（当前 v${result.current}）`,
      };
    } else {
      updateResult.value = {
        type: "ok",
        text: `已是最新版本 v${result.current}`,
      };
    }
  } catch (e) {
    updateResult.value = { type: "err", text: `检查失败：${String(e)}` };
  } finally {
    checkingUpdate.value = false;
  }
}

function stageLabel(t: StageType) {
  return t === "work" ? "工作" : "休息";
}
function stageColor(t: StageType) {
  return t === "work" ? "#185FA5" : "#0F6E56";
}
</script>

<template>
  <div class="settings" v-if="config">
    <h1>FocusLock 配置</h1>

    <!-- 阶段列表 -->
    <section>
      <div class="section-head">
        <h2>阶段循环</h2>
        <div class="actions">
          <button class="btn-sm" @click="addStage('work')">+ 工作</button>
          <button class="btn-sm" @click="addStage('rest')">+ 休息</button>
        </div>
      </div>
      <p class="hint">按顺序循环执行。至少需要一个工作阶段。</p>

      <div class="stage-list">
        <div
          v-for="(s, i) in config.stages"
          :key="i"
          class="stage-item"
          :style="{ borderLeftColor: stageColor(s.type) }"
        >
          <span class="stage-badge" :style="{ background: stageColor(s.type) }">
            {{ stageLabel(s.type) }}
          </span>
          <select v-model="s.type" class="stage-select">
            <option value="work">工作</option>
            <option value="rest">休息</option>
          </select>
          <input type="number" min="1" max="180" v-model.number="s.minutes" class="stage-min" />
          <span class="unit">分钟</span>
          <div class="stage-ops">
            <button class="btn-icon" @click="moveUp(i)" :disabled="i === 0" title="上移">↑</button>
            <button class="btn-icon" @click="moveDown(i)" :disabled="i === config.stages.length - 1" title="下移">↓</button>
            <button class="btn-icon danger" @click="removeStage(i)" title="删除">×</button>
          </div>
        </div>
      </div>
      <p v-if="!hasWork" class="err">至少需要一个工作阶段</p>
    </section>

    <!-- 提醒模式 -->
    <section>
      <h2>休息提醒方式</h2>
      <label class="radio">
        <input type="radio" v-model="config.rest_reminder_mode" value="fullscreen" />
        全屏遮罩（所有显示器，软强制）
      </label>
      <label class="radio">
        <input type="radio" v-model="config.rest_reminder_mode" value="popup" />
        弹窗提示（右下角小窗，不阻断）
      </label>
    </section>

    <!-- 遮罩样式（仅 fullscreen 时有意义） -->
    <section v-if="config.rest_reminder_mode === 'fullscreen'">
      <h2>遮罩样式</h2>
      <div class="style-options">
        <label class="radio style-option" :class="{ active: config.overlay_style === 'semi_transparent' }">
          <input type="radio" v-model="config.overlay_style" value="semi_transparent" />
          <div class="style-preview semi-transparent-preview">半透明黑底</div>
          <span>半透明（默认）</span>
        </label>
        <label class="radio style-option" :class="{ active: config.overlay_style === 'full_black' }">
          <input type="radio" v-model="config.overlay_style" value="full_black" />
          <div class="style-preview full-black-preview">纯黑不透明</div>
          <span>全黑</span>
        </label>
        <label class="radio style-option" :class="{ active: config.overlay_style === 'dark' }">
          <input type="radio" v-model="config.overlay_style" value="dark" />
          <div class="style-preview dark-preview">暗色调</div>
          <span>暗色</span>
        </label>
      </div>
    </section>

    <!-- 自定义休息提示词 -->
    <section>
      <h2>休息提示词</h2>
      <div class="field">
        <input
          type="text"
          v-model="config.rest_message"
          placeholder="现在休息"
          maxlength="20"
          class="message-input"
        />
        <span class="unit">{{ config.rest_message.length }}/20 字</span>
      </div>
      <p class="hint">休息遮罩顶部显示的自定义文字，留空默认显示「现在休息」</p>
    </section>

    <!-- 数值参数 -->
    <section>
      <h2>计时参数</h2>
      <div class="field">
        <label>长时间离开重置阈值</label>
        <input type="number" min="1" v-model.number="config.reset_threshold_minutes" />
        <span class="unit">分钟</span>
      </div>
      <div class="field">
        <label>工作结束前提醒</label>
        <input type="number" min="0" max="30" v-model.number="config.notify_before_work_end_minutes" />
        <span class="unit">分钟</span>
      </div>
    </section>

    <!-- 快捷键 -->
    <section>
      <h2>跳过休息快捷键</h2>
      <div class="field">
        <input type="text" v-model="config.skip_shortcut" placeholder="CmdOrCtrl+Shift+F2" />
        <span class="unit">Tauri accelerator 格式</span>
      </div>
      <p class="hint">示例：{{ isMac ? "Cmd+Shift+F2" : "Ctrl+Shift+F2" }}（用 CmdOrCtrl 跨平台自动映射）</p>
    </section>

    <!-- Windows 管理员自启 -->
    <section v-if="!isMac">
      <h2>开机自启（高级）</h2>
      <label class="radio">
        <input type="checkbox" v-model="config.run_as_admin_autostart" />
        以管理员权限自启（提升遮罩覆盖能力，每次开机弹 UAC）
      </label>
      <p class="hint">默认关闭。开启后可覆盖任务管理器等管理员程序，代价是开机时弹 UAC 确认。</p>
    </section>

    <!-- 检查更新 -->
    <section>
      <h2>关于与更新</h2>
      <div class="update-bar">
        <button
          class="btn-secondary"
          :disabled="checkingUpdate"
          @click="onCheckUpdate"
        >
          {{ checkingUpdate ? "检查中…" : "检查更新" }}
        </button>
        <a href="https://github.com/weifeng-work/FocusLock/releases" target="_blank" class="update-link">
          GitHub 发布页 →
        </a>
      </div>
      <div v-if="updateResult" :class="['msg', 'inline', updateResult.type]">{{ updateResult.text }}</div>
    </section>

    <!-- 操作 -->
    <section class="actions-bar">
      <button class="btn-primary" :disabled="saving || !stagesValid" @click="onSave">
        {{ saving ? "保存中…" : "保存配置" }}
      </button>
      <button class="btn-secondary" @click="onResetTimer">重置计时（使新配置生效）</button>
    </section>

    <div v-if="message" :class="['msg', message.type]">{{ message.text }}</div>

    <!-- 微信客服群 -->
    <section class="footer-section">
      <div class="wechat-section">
        <p class="wechat-hint">扫码加入 FocusLock 微信客服群，获取帮助 / 提反馈 / 参与讨论</p>
        <img src="/wechat-qrcode.png" alt="FocusLock 微信客服群" class="wechat-qr" />
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings {
  max-width: 560px;
  margin: 0 auto;
  padding: 24px 20px 48px;
  color: #2c2c2a;
}
h1 {
  font-size: 22px;
  font-weight: 500;
  margin: 0 0 24px;
}
h2 {
  font-size: 15px;
  font-weight: 500;
  margin: 0 0 8px;
}
section {
  margin-bottom: 24px;
  padding-bottom: 16px;
  border-bottom: 0.5px solid #d3d1c7;
}
.section-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.actions {
  display: flex;
  gap: 6px;
}
.hint {
  font-size: 12px;
  color: #5f5e5a;
  margin: 4px 0 8px;
}
.err {
  color: #993c1d;
  font-size: 12px;
}
.stage-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.stage-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  background: #f1efe8;
  border-radius: 6px;
  border-left: 3px solid #185fa5;
}
.stage-badge {
  color: #fff;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  font-weight: 500;
}
.stage-select,
.stage-min {
  padding: 4px 6px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-size: 13px;
}
.stage-min {
  width: 56px;
}
.unit {
  font-size: 12px;
  color: #5f5e5a;
}
.stage-ops {
  margin-left: auto;
  display: flex;
  gap: 2px;
}
.btn-sm {
  padding: 4px 10px;
  font-size: 12px;
  border: 0.5px solid #888780;
  background: transparent;
  border-radius: 4px;
  cursor: pointer;
}
.btn-sm:hover {
  background: #e6f1fb;
}
.btn-icon {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 14px;
  border-radius: 4px;
}
.btn-icon:hover:not(:disabled) {
  background: #d3d1c7;
}
.btn-icon.danger:hover {
  background: #faece7;
  color: #993c1d;
}
.btn-icon:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}
.radio {
  display: block;
  padding: 6px 0;
  font-size: 13px;
  cursor: pointer;
}
.field {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 6px 0;
}
.field label {
  font-size: 13px;
  min-width: 140px;
}
.field input[type="number"] {
  width: 64px;
  padding: 4px 6px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
}
.field input[type="text"] {
  flex: 1;
  padding: 4px 6px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-family: ui-monospace, monospace;
}
.message-input {
  max-width: 200px !important;
  font-family: inherit !important;
}
.actions-bar {
  display: flex;
  gap: 10px;
  border-bottom: none;
}
.btn-primary {
  padding: 8px 18px;
  background: #185fa5;
  color: #fff;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
}
.btn-primary:disabled {
  opacity: 0.5;
}
.btn-secondary {
  padding: 8px 14px;
  background: transparent;
  color: #2c2c2a;
  border: 0.5px solid #888780;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
}
.msg {
  margin-top: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 12px;
}
.msg.ok {
  background: #e1f5ee;
  color: #085041;
}
.msg.warn {
  background: #faeeda;
  color: #633806;
}
.msg.err {
  background: #faece7;
  color: #993c1d;
}
.msg.info {
  background: #e8ecf4;
  color: #334e7a;
}
.msg.inline {
  display: inline-block;
  margin-top: 8px;
}

/* 遮罩样式预览 */
.style-options {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}
.style-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px;
  border-radius: 8px;
  border: 1.5px solid transparent;
  transition: all 0.15s;
}
.style-option.active {
  border-color: #185fa5;
  background: #f0f6fc;
}
.style-option input[type="radio"] {
  display: none;
}
.style-preview {
  width: 72px;
  height: 48px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 9px;
  color: #fff;
  font-weight: 500;
}
.semi-transparent-preview {
  background: rgba(0, 0, 0, 0.85);
  /* 用棋盘格暗示透明 */
  background-image:
    linear-gradient(45deg, rgba(60, 60, 60, 0.15) 25%, transparent 25%),
    linear-gradient(-45deg, rgba(60, 60, 60, 0.15) 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, rgba(60, 60, 60, 0.15) 75%),
    linear-gradient(-45deg, transparent 75%, rgba(60, 60, 60, 0.15) 75%);
  background-size: 8px 8px;
  background-color: rgba(0, 0, 0, 0.88);
  color: rgba(255, 255, 255, 0.8);
}
.full-black-preview {
  background: #000000;
}
.dark-preview {
  background: #121218;
}

/* 检查更新 */
.update-bar {
  display: flex;
  align-items: center;
  gap: 12px;
}
.update-link {
  font-size: 12px;
  color: #185fa5;
  text-decoration: none;
}
.update-link:hover {
  text-decoration: underline;
}

/* 底部微信区域 */
.footer-section {
  border-bottom: none;
  text-align: center;
}
.wechat-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}
.wechat-hint {
  font-size: 12px;
  color: #5f5e5a;
  margin: 0;
}
.wechat-qr {
  width: 160px;
  height: 160px;
  border: 1px solid #d3d1c7;
  border-radius: 8px;
  object-fit: contain;
}
</style>
