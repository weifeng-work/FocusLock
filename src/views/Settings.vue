<script setup lang="ts">
// 配置面板（阶段 6）
// - 阶段列表编辑器：增删/上下移/改类型/改分钟
// - rest_reminder_mode 单选（fullscreen / popup）
// - reset_threshold_minutes、notify_before_work_end_minutes 数字输入
// - skip_shortcut 文本输入（Tauri accelerator 格式）
// - Windows 专属：run_as_admin_autostart 开关
// - 保存 → invoke("save_config") → 提示需重置/重启生效

import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Config, Stage, StageType, RestReminderMode } from "../types";

const config = ref<Config | null>(null);
const saving = ref(false);
const message = ref<{ type: "ok" | "warn" | "err"; text: string } | null>(null);
const isMac = navigator.platform.toUpperCase().includes("MAC");

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

    <!-- 操作 -->
    <section class="actions-bar">
      <button class="btn-primary" :disabled="saving || !stagesValid" @click="onSave">
        {{ saving ? "保存中…" : "保存配置" }}
      </button>
      <button class="btn-secondary" @click="onResetTimer">重置计时（使新配置生效）</button>
    </section>

    <div v-if="message" :class="['msg', message.type]">{{ message.text }}</div>
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
</style>
