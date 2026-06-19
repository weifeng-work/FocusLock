<script setup lang="ts">
// 配置面板（v2 - 支持多方案）
// - 方案管理：选择/编辑当前方案
// - 阶段列表编辑器：增删/上下移/改类型/改分钟
// - rest_reminder_mode 单选（fullscreen / popup）
// - overlay_style 选择：半透明 / 全黑 / 暗色
// - 提示音设置
// - 全局设置：reset_threshold_minutes、notify_before_work_end_minutes 等
// - 保存 → invoke("save_config") → 提示需重置/重启生效

import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../locales";
import type { Config, Scheme, Stage, StageType, RestReminderMode, OverlayStyle, SoundType } from "../types";

const { currentLocale, setLocale, t } = useI18n();
const config = ref<Config | null>(null);
const saving = ref(false);
const message = ref<{ type: "ok" | "warn" | "err"; text: string } | null>(null);
const isMac = navigator.platform.toUpperCase().includes("MAC");

// 当前选中的方案索引
const currentSchemeIndex = ref(0);

// 音效相关状态
const soundFiles = ref<{ name: string; file: string; path: string }[]>([]);
const uploading = ref(false);

// 检查更新相关状态
const checkingUpdate = ref(false);
const updateResult = ref<{ type: "ok" | "warn" | "err" | "info"; text: string } | null>(null);

// 获取当前方案
const currentScheme = computed<Scheme | undefined>(() => {
  return config.value?.schemes[currentSchemeIndex.value];
});

async function load() {
  config.value = await invoke<Config>("get_config");
  await loadSoundFiles();
}

async function loadSoundFiles() {
  try {
    soundFiles.value = await invoke<any[]>("get_sound_files");
  } catch (e) {
    console.warn("加载音效文件失败:", e);
  }
}

onMounted(load);

function onLanguageChange() {
  setLocale(currentLocale.value);
}

// 阶段操作
function addStage(type: StageType) {
  if (!currentScheme.value) return;
  currentScheme.value.stages.push({ type, minutes: type === "work" ? 25 : 5 });
}

function removeStage(i: number) {
  if (!currentScheme.value) return;
  currentScheme.value.stages.splice(i, 1);
}

function moveUp(i: number) {
  if (!currentScheme.value || i <= 0) return;
  const arr = currentScheme.value.stages;
  [arr[i - 1], arr[i]] = [arr[i], arr[i - 1]];
}

function moveDown(i: number) {
  if (!currentScheme.value || i >= currentScheme.value.stages.length - 1) return;
  const arr = currentScheme.value.stages;
  [arr[i + 1], arr[i]] = [arr[i], arr[i + 1]];
}

const hasWork = computed(() => currentScheme.value?.stages.some((s) => s.type === "work") ?? false);
const stagesValid = computed(() => (currentScheme.value?.stages.length ?? 0) > 0 && hasWork.value);

async function onSave() {
  if (!config.value) {
    message.value = { type: "err", text: "配置无效" };
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

// 音效相关方法
function getSoundTypeValue(sound: SoundType): string {
  if (sound === "none") return "none";
  if (sound === "builtin") return "builtin";
  if (typeof sound === "object" && sound !== null && "custom" in sound) {
    return (sound as { custom: string }).custom;
  }
  return "none";
}

function setSchemeSoundType(soundRef: "work_end_sound" | "rest_end_sound", value: string) {
  if (!currentScheme.value) return;
  if (value === "none") {
    currentScheme.value[soundRef] = "none";
  } else if (value === "builtin") {
    currentScheme.value[soundRef] = "builtin";
  } else {
    currentScheme.value[soundRef] = { custom: value } as SoundType;
  }
}

function getSoundDisplayName(sound: SoundType): string {
  if (sound === "none") return "无声";
  if (sound === "builtin") return "内置提示音";
  if (typeof sound === "object" && sound !== null && "custom" in sound) {
    const custom = (sound as { custom: string }).custom;
    const found = soundFiles.value.find(f => f.file === custom);
    return found ? found.name : custom;
  }
  return "未知";
}

async function onUploadSound() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const filePath = await open({
      multiple: false,
      filters: [{
        name: "Audio Files",
        extensions: ["mp3", "wav", "aac", "ogg", "flac", "m4a"],
      }],
    });
    if (!filePath) return;

    uploading.value = true;
    const result = await invoke<{ name: string; file: string; path: string }>("copy_custom_sound", {
      sourcePath: filePath,
    });
    await loadSoundFiles();
    message.value = { type: "ok", text: `已上传音效「${result.name}」` };
  } catch (e) {
    message.value = { type: "err", text: `上传失败：${String(e)}` };
  } finally {
    uploading.value = false;
  }
}

async function onDeleteSound(file: string) {
  if (!confirm(`确定要删除「${file}」吗？`)) return;
  try {
    await invoke("delete_sound_file", { fileName: file });
    await loadSoundFiles();
    message.value = { type: "ok", text: `已删除音效「${file}」` };
  } catch (e) {
    message.value = { type: "err", text: `删除失败：${String(e)}` };
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
    <div class="settings-header">
      <h1>{{ t("settings.title") }}</h1>
      <div class="language-switch">
        <select v-model="currentLocale" @change="onLanguageChange">
          <option value="zh">{{ t("tray.languageZh") }}</option>
          <option value="en">{{ t("tray.languageEn") }}</option>
        </select>
      </div>
    </div>

    <!-- 方案选择 -->
    <section>
      <h2>当前方案</h2>
      <select v-model="currentSchemeIndex" class="scheme-select">
        <option v-for="(scheme, index) in config.schemes" :key="scheme.id" :value="index">
          {{ scheme.name }}
        </option>
      </select>
    </section>

    <!-- 阶段列表 -->
    <section v-if="currentScheme">
      <div class="section-head">
        <h2>{{ t("settings.stages") }}</h2>
        <div class="actions">
          <button class="btn-sm" @click="addStage('work')">{{ t("settings.addStage") }} {{ t("settings.stageWork") }}</button>
          <button class="btn-sm" @click="addStage('rest')">{{ t("settings.addStage") }} {{ t("settings.stageRest") }}</button>
        </div>
      </div>
      <p class="hint">{{ t("settings.stagesHint") || "按顺序循环执行。至少需要一个工作阶段。" }}</p>

      <div class="stage-list">
        <div
          v-for="(s, i) in currentScheme.stages"
          :key="i"
          class="stage-item"
          :style="{ borderLeftColor: stageColor(s.type) }"
        >
          <span class="stage-badge" :style="{ background: stageColor(s.type) }">
            {{ stageLabel(s.type) }}
          </span>
          <select v-model="s.type" class="stage-select">
            <option value="work">{{ t("settings.stageWork") || "工作" }}</option>
            <option value="rest">{{ t("settings.stageRest") || "休息" }}</option>
          </select>
          <input type="number" min="1" max="180" v-model.number="s.minutes" class="stage-min" />
          <span class="unit">{{ t("settings.minutes") || "分钟" }}</span>
          <div class="stage-ops">
          <button class="btn-icon" @click="moveUp(i)" :disabled="i === 0" :title="t('settings.moveUp') || '上移'">↑</button>
          <button class="btn-icon" @click="moveDown(i)" :disabled="i === currentScheme.stages.length - 1" :title="t('settings.moveDown') || '下移'">↓</button>
          <button class="btn-icon danger" @click="removeStage(i)" :title="t('settings.removeStage') || '删除'">×</button>
          </div>
        </div>
      </div>
      <p v-if="!hasWork" class="err">{{ t('settings.noWorkStage') || '至少需要一个工作阶段' }}</p>
    </section>

    <!-- 提醒模式 -->
    <section v-if="currentScheme">
      <h2>{{ t("settings.restReminder") }}</h2>
      <label class="radio">
        <input type="radio" v-model="currentScheme.rest_reminder_mode" value="fullscreen" />
        {{ t("settings.modeFullscreen") }}
      </label>
      <label class="radio">
        <input type="radio" v-model="currentScheme.rest_reminder_mode" value="popup" />
        {{ t("settings.modePopup") }}
      </label>
    </section>

    <!-- 提示音 -->
    <section v-if="currentScheme">
      <h2>{{ t("settings.sound") || "提示音" }}</h2>
      <p class="hint">{{ t("settings.soundHint") || "设置工作结束（进入休息）和休息结束（返回工作）时的提示音。" }}</p>

      <!-- 工作结束提示音 -->
      <div class="sound-section">
        <h3>{{ t("settings.workEndSound") || "工作结束提示音（进入休息）" }}</h3>
        <div class="sound-options">
          <label class="radio">
            <input
              type="radio"
              :checked="getSoundTypeValue(currentScheme.work_end_sound) === 'none'"
              @change="setSchemeSoundType('work_end_sound', 'none')"
            />
            {{ t("settings.soundNone") || "无声" }}
          </label>
          <label class="radio">
            <input
              type="radio"
              :checked="getSoundTypeValue(currentScheme.work_end_sound) === 'builtin'"
              @change="setSchemeSoundType('work_end_sound', 'builtin')"
            />
            {{ t("settings.soundBuiltin") || "内置提示音" }}
          </label>
          <label class="radio" v-if="soundFiles.length > 0">
            {{ t("settings.soundCustom") || "自定义音频" }}：
            <select
              :value="getSoundTypeValue(currentScheme.work_end_sound)"
              @change="(e: Event) => setSchemeSoundType('work_end_sound', (e.target as HTMLSelectElement).value)"
            >
              <option value="none">-- {{ t("settings.selectSoundFile") || "选择音频" }} --</option>
              <option v-for="f in soundFiles" :key="f.file" :value="f.file">
                {{ f.name }}
              </option>
            </select>
          </label>
        </div>
      </div>

      <!-- 休息结束提示音 -->
      <div class="sound-section">
        <h3>{{ t("settings.restEndSound") || "休息结束提示音（返回工作）" }}</h3>
        <div class="sound-options">
          <label class="radio">
            <input
              type="radio"
              :checked="getSoundTypeValue(currentScheme.rest_end_sound) === 'none'"
              @change="setSchemeSoundType('rest_end_sound', 'none')"
            />
            {{ t("settings.soundNone") || "无声" }}
          </label>
          <label class="radio">
            <input
              type="radio"
              :checked="getSoundTypeValue(currentScheme.rest_end_sound) === 'builtin'"
              @change="setSchemeSoundType('rest_end_sound', 'builtin')"
            />
            {{ t("settings.soundBuiltin") || "内置提示音" }}
          </label>
          <label class="radio" v-if="soundFiles.length > 0">
            {{ t("settings.soundCustom") || "自定义音频" }}：
            <select
              :value="getSoundTypeValue(currentScheme.rest_end_sound)"
              @change="(e: Event) => setSchemeSoundType('rest_end_sound', (e.target as HTMLSelectElement).value)"
            >
              <option value="none">-- {{ t("settings.selectSoundFile") || "选择音频" }} --</option>
              <option v-for="f in soundFiles" :key="f.file" :value="f.file">
                {{ f.name }}
              </option>
            </select>
          </label>
        </div>
      </div>

      <!-- 上传自定义音效 -->
      <div class="sound-upload">
        <button class="btn-sm" @click="onUploadSound" :disabled="uploading">
          {{ uploading ? (t("settings.uploading") || "上传中…") : (t("settings.uploadSound") || "上传自定义音效") }}
        </button>
        <span class="hint">{{ t("settings.soundFileHint") || "支持 mp3/wav/aac/ogg/flac/m4a 格式" }}</span>
      </div>

      <!-- 已上传音效列表 -->
      <div v-if="soundFiles.length > 0" class="sound-list">
        <h3>{{ t("settings.uploadedSounds") || "已上传的音效" }}</h3>
        <div v-for="f in soundFiles" :key="f.file" class="sound-item">
          <span>{{ f.name }}</span>
          <button class="btn-icon danger" @click="onDeleteSound(f.file)">{{ t("settings.deleteSound") || "删除" }}</button>
        </div>
      </div>
    </section>

    <!-- 数值参数 -->
    <section>
      <h2>{{ t("settings.system") || "系统设置" }}</h2>
      <div class="field">
        <label>{{ t("settings.resetThreshold") || "长时间离开重置阈值" }}</label>
        <input type="number" min="1" v-model.number="config.reset_threshold_minutes" />
        <span class="unit">分钟</span>
      </div>
      <div class="field">
        <label>{{ t("settings.notifyBefore") || "工作结束前提醒" }}</label>
        <input type="number" min="0" max="30" v-model.number="config.notify_before_work_end_minutes" />
        <span class="unit">分钟</span>
      </div>
    </section>

    <!-- 快捷键 -->
    <section>
      <h2>{{ t("settings.skipShortcut") || "跳过休息快捷键" }}</h2>
      <div class="field">
        <input type="text" v-model="config.skip_shortcut" placeholder="CmdOrCtrl+Shift+F2" />
        <span class="unit">Tauri accelerator 格式</span>
      </div>
      <p class="hint">{{ t("settings.shortcutHint") || "示例：{shortcut}（用 CmdOrCtrl 跨平台自动映射）" }}</p>
    </section>

    <!-- Windows 管理员自启 -->
    <section v-if="!isMac">
      <h2>{{ t("settings.runAsAdmin") || "开机自启（高级）" }}</h2>
      <label class="radio">
        <input type="checkbox" v-model="config.run_as_admin_autostart" />
        以管理员权限自启（提升遮罩覆盖能力，每次开机弹 UAC）
      </label>
      <p class="hint">{{ t("settings.adminHint") || "默认关闭。开启后可覆盖任务管理器等管理员程序，代价是开机时弹 UAC 确认。" }}</p>
    </section>

    <!-- 检查更新 -->
    <section>
      <h2>{{ t("settings.about") || "关于与更新" }}</h2>
      <div class="update-bar">
        <button
          class="btn-secondary"
          :disabled="checkingUpdate"
          @click="onCheckUpdate"
        >
          {{ checkingUpdate ? (t("settings.checking") || "检查中…") : (t("settings.checkUpdate") || "检查更新") }}
        </button>
        <a href="https://github.com/weifeng-work/FocusLock/releases" target="_blank" class="update-link">
          {{ t("settings.githubRelease") || "GitHub 发布页" }} →
        </a>
      </div>
      <div v-if="updateResult" :class="['msg', 'inline', updateResult.type]">{{ updateResult.text }}</div>
    </section>

    <!-- 操作 -->
    <section class="actions-bar">
      <button class="btn-primary" :disabled="saving || !stagesValid" @click="onSave">
          {{ saving ? (t("settings.saving") || "保存中…") : (t("settings.save") || "保存配置") }}
        </button>
        <button class="btn-secondary" @click="onResetTimer">{{ t("settings.resetTimer") || "重置计时（使新配置生效）" }}</button>
    </section>

    <div v-if="message" :class="['msg', message.type]">{{ message.text }}</div>

    <!-- 微信客服群 -->
    <section class="footer-section">
      <div class="wechat-section">
        <p class="wechat-hint">{{ t("settings.wechatHint") || "扫码加入 FocusLock 微信客服群，获取帮助 / 提反馈 / 参与讨论" }}</p>
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
  background: #faeda;
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

/* 设置面板头部 */
.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.settings-header h1 {
  margin: 0;
}
.language-switch select {
  padding: 4px 8px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-size: 13px;
  background: transparent;
  cursor: pointer;
}

/* 方案选择 */
.scheme-select {
  padding: 6px 10px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-size: 14px;
  min-width: 200px;
}

/* 提示音设置 */
.sound-section {
  margin: 12px 0;
  padding: 10px;
  background: #f7f6f2;
  border-radius: 6px;
}
.sound-section h3 {
  font-size: 13px;
  font-weight: 500;
  margin: 0 0 8px;
  color: #3a3935;
}
.sound-options {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.sound-options label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  cursor: pointer;
}
.sound-options select {
  padding: 3px 6px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-size: 12px;
  max-width: 160px;
}
.sound-upload {
  margin: 12px 0 8px;
  display: flex;
  align-items: center;
  gap: 10px;
}
.sound-list {
  margin-top: 10px;
  padding: 8px;
  background: #f1efe8;
  border-radius: 6px;
}
.sound-list h3 {
  font-size: 12px;
  font-weight: 500;
  margin: 0 0 6px;
  color: #5f5e5a;
}
.sound-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0;
  font-size: 12px;
}
</style>
