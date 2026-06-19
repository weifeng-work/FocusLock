<script setup lang="ts">
// FocusLock 设置面板（v2 - 完整版）
// Tab 1: 方案管理 - 多方案 CRUD、阶段编辑、提示音、休息提醒模式
// Tab 2: 作息表 - 多 Routine CRUD、时间段（TimePeriod）CRUD、end_action
// Tab 3: 周配置 - 每天分配一个 routine
// Tab 4: 系统设置 - 阈值、通知时间、快捷键、自启、检查更新

import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../locales";
import type {
  Config, Scheme, Stage, StageType, Routine, TimePeriod,
  PeriodEndAction, OverlayStyle, SoundType, WeeklyAssignment,
} from "../types";

const { currentLocale, setLocale, t } = useI18n();

const config = ref<Config | null>(null);
const activeTab = ref<"scheme" | "routine" | "weekly" | "system">("scheme");
const saving = ref(false);
const message = ref<{ type: "ok" | "warn" | "err"; text: string } | null>(null);
const isMac = navigator.platform.toUpperCase().includes("MAC");

// 当前选中的方案索引（在 Tab 1 中）
const currentSchemeIndex = ref(0);

// 音效相关
const soundFiles = ref<{ name: string; file: string; path: string }[]>([]);
const uploading = ref(false);

// 检查更新
const checkingUpdate = ref(false);
const updateResult = ref<{ type: "ok" | "warn" | "err" | "info"; text: string } | null>(null);

// ============== 加载 ==============
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

const currentScheme = computed<Scheme | undefined>(() => {
  return config.value?.schemes[currentSchemeIndex.value];
});

function isBuiltInScheme(scheme: Scheme): boolean {
  // 内置方案 id: pomodoro / standard / deep_work
  return ["pomodoro", "standard", "deep_work"].includes(scheme.id);
}

// ============== 语言切换 ==============
function onLanguageChange() {
  setLocale(currentLocale.value);
}

// ============== 方案 CRUD ==============
function genId(prefix: string): string {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
}

function createDefaultScheme(): Scheme {
  return {
    id: genId("scheme"),
    name: t("settings.scheme.newScheme") || "新建方案",
    stages: [
      { type: "work", minutes: 25 },
      { type: "rest", minutes: 5 },
    ],
    rest_reminder_mode: "fullscreen",
    work_end_sound: "builtin",
    rest_end_sound: "builtin",
  };
}

function onNewScheme() {
  if (!config.value) return;
  config.value.schemes.push(createDefaultScheme());
  currentSchemeIndex.value = config.value.schemes.length - 1;
}

function onCloneScheme() {
  if (!config.value || !currentScheme.value) return;
  const cloned: Scheme = JSON.parse(JSON.stringify(currentScheme.value));
  cloned.id = genId("scheme");
  cloned.name = `${cloned.name} (副本)`;
  config.value.schemes.push(cloned);
  currentSchemeIndex.value = config.value.schemes.length - 1;
}

function onDeleteScheme() {
  if (!config.value || !currentScheme.value) return;
  const scheme = currentScheme.value;
  if (isBuiltInScheme(scheme)) {
    message.value = { type: "warn", text: t("settings.scheme.builtInWarnDelete") || "内置方案不可删除" };
    return;
  }
  // 检查是否被 routine 引用
  const referenced = (config.value.routines || []).some(r =>
    r.periods.some(p => p.scheme_id === scheme.id)
  );
  if (referenced) {
    message.value = {
      type: "err",
      text: t("settings.scheme.deleteSchemeConfirm", { name: scheme.name }) ||
        "方案被引用，无法删除"
    };
    return;
  }
  if (!confirm(t("settings.scheme.deleteSchemeConfirm", { name: scheme.name }) || `确定要删除「${scheme.name}」吗？`)) return;
  const idx = currentSchemeIndex.value;
  config.value.schemes.splice(idx, 1);
  if (currentSchemeIndex.value >= config.value.schemes.length) {
    currentSchemeIndex.value = Math.max(0, config.value.schemes.length - 1);
  }
}

function onRenameScheme() {
  if (!currentScheme.value) return;
  if (isBuiltInScheme(currentScheme.value)) {
    message.value = { type: "warn", text: t("settings.scheme.builtInWarnEditName") || "内置方案不可重命名" };
    return;
  }
  const newName = prompt("方案名：", currentScheme.value.name);
  if (newName && newName.trim()) {
    currentScheme.value.name = newName.trim();
  }
}

// ============== 阶段 CRUD ==============
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

// ============== 提示音辅助 ==============
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
  if (value === "none") currentScheme.value[soundRef] = "none";
  else if (value === "builtin") currentScheme.value[soundRef] = "builtin";
  else currentScheme.value[soundRef] = { custom: value } as SoundType;
}

async function onUploadSound() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const filePath = await open({
      multiple: false,
      filters: [{ name: "Audio Files", extensions: ["mp3", "wav", "aac", "ogg", "flac", "m4a"] }],
    });
    if (!filePath) return;
    uploading.value = true;
    const result = await invoke<{ name: string; file: string; path: string }>("copy_custom_sound", { sourcePath: filePath });
    await loadSoundFiles();
    message.value = { type: "ok", text: `已上传「${result.name}」` };
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
  } catch (e) {
    message.value = { type: "err", text: `删除失败：${String(e)}` };
  }
}

// ============== 作息表 CRUD ==============
const currentRoutineIndex = ref(0);

const currentRoutine = computed<Routine | undefined>(() => {
  return config.value?.routines[currentRoutineIndex.value];
});

function createDefaultRoutine(): Routine {
  return {
    id: genId("routine"),
    name: "新建作息表",
    periods: [],
  };
}

function onNewRoutine() {
  if (!config.value) return;
  config.value.routines.push(createDefaultRoutine());
  currentRoutineIndex.value = config.value.routines.length - 1;
}

function onDeleteRoutine() {
  if (!config.value || !currentRoutine.value) return;
  const routine = currentRoutine.value;
  // 检查是否被周配置引用
  const usedInWeekly = Object.values(config.value.weekly || {}).includes(routine.id);
  if (usedInWeekly) {
    message.value = { type: "err", text: "该作息表已被周配置引用，请先修改周配置" };
    return;
  }
  if (!confirm(t("settings.routine.deleteRoutineConfirm", { name: routine.name }) || `确定要删除「${routine.name}」吗？`)) return;
  const idx = currentRoutineIndex.value;
  config.value.routines.splice(idx, 1);
  if (currentRoutineIndex.value >= config.value.routines.length) {
    currentRoutineIndex.value = Math.max(0, config.value.routines.length - 1);
  }
}

function onRenameRoutine() {
  if (!currentRoutine.value) return;
  const newName = prompt("作息表名：", currentRoutine.value.name);
  if (newName && newName.trim()) {
    currentRoutine.value.name = newName.trim();
  }
}

// 时间段 CRUD
function createDefaultPeriod(schemeId: string): TimePeriod {
  return {
    id: genId("period"),
    start_hour: 8,
    start_minute: 0,
    end_hour: 12,
    end_minute: 0,
    scheme_id: schemeId,
    end_action: { type: "none" },
  };
}

function onNewPeriod() {
  if (!currentRoutine.value || !config.value || config.value.schemes.length === 0) return;
  const firstScheme = config.value.schemes[0];
  currentRoutine.value.periods.push(createDefaultPeriod(firstScheme.id));
}

function onDeletePeriod(i: number) {
  if (!currentRoutine.value) return;
  currentRoutine.value.periods.splice(i, 1);
}

function onMovePeriodUp(i: number) {
  if (!currentRoutine.value || i <= 0) return;
  const arr = currentRoutine.value.periods;
  [arr[i - 1], arr[i]] = [arr[i], arr[i - 1]];
}

function onMovePeriodDown(i: number) {
  if (!currentRoutine.value || i >= currentRoutine.value.periods.length - 1) return;
  const arr = currentRoutine.value.periods;
  [arr[i + 1], arr[i]] = [arr[i], arr[i + 1]];
}

// ============== 周配置 ==============
const weekdayKeys: Array<{ key: keyof WeeklyAssignment; label: string }> = [
  { key: "monday", label: "mon" },
  { key: "tuesday", label: "tue" },
  { key: "wednesday", label: "wed" },
  { key: "thursday", label: "thu" },
  { key: "friday", label: "fri" },
  { key: "saturday", label: "sat" },
  { key: "sunday", label: "sun" },
];

function onSetWeekday(key: keyof WeeklyAssignment, routineId: string) {
  if (!config.value) return;
  config.value.weekly[key] = routineId;
}

// ============== end_action 辅助 ==============
function setEndActionType(period: TimePeriod, type: "none" | "popup" | "fullscreen" | "black_screen") {
  const oldText = period.end_action.type !== "none" ? (period.end_action as any).text : "";
  const oldSound = period.end_action.type !== "none" ? (period.end_action as any).sound : ("builtin" as SoundType);
  const oldStyle = period.end_action.type === "fullscreen" ? (period.end_action as any).style : "semi_transparent" as OverlayStyle;

  if (type === "none") {
    period.end_action = { type: "none" };
  } else if (type === "popup") {
    period.end_action = { type: "popup", text: oldText, sound: oldSound };
  } else if (type === "fullscreen") {
    period.end_action = { type: "fullscreen", text: oldText, sound: oldSound, style: oldStyle };
  } else if (type === "black_screen") {
    period.end_action = { type: "black_screen", text: oldText, sound: oldSound };
  }
}

function setEndActionText(period: TimePeriod, value: string) {
  if (period.end_action.type === "none") return;
  (period.end_action as any).text = value;
}

function setEndActionSound(period: TimePeriod, value: string) {
  if (period.end_action.type === "none") return;
  let s: SoundType;
  if (value === "none") s = "none";
  else if (value === "builtin") s = "builtin";
  else s = { custom: value } as SoundType;
  (period.end_action as any).sound = s;
}

function setEndActionStyle(period: TimePeriod, value: OverlayStyle) {
  if (period.end_action.type !== "fullscreen") return;
  (period.end_action as any).style = value;
}

// ============== 保存 ==============
async function onSave() {
  if (!config.value) {
    message.value = { type: "err", text: "配置无效" };
    return;
  }
  // 校验
  if (!config.value.schemes.length) {
    message.value = { type: "err", text: "至少需要一个方案" };
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
      updateResult.value = { type: "warn", text: `发现新版本 ${result.latest}（当前 v${result.current}）` };
    } else {
      updateResult.value = { type: "ok", text: `已是最新版本 v${result.current}` };
    }
  } catch (e) {
    updateResult.value = { type: "err", text: `检查失败：${String(e)}` };
  } finally {
    checkingUpdate.value = false;
  }
}

// ============== 工具 ==============
function stageLabel(stageType: StageType) {
  return stageType === "work" ? t("settings.stageWork") : t("settings.stageRest");
}
function stageColor(stageType: StageType) {
  return stageType === "work" ? "#185FA5" : "#0F6E56";
}
function periodActionType(period: TimePeriod): string {
  return period.end_action.type;
}
function periodActionText(period: TimePeriod): string {
  if (period.end_action.type === "none") return "";
  return (period.end_action as any).text || "";
}
function periodActionSound(period: TimePeriod): string {
  if (period.end_action.type === "none") return "none";
  return getSoundTypeValue((period.end_action as any).sound);
}
function periodActionStyle(period: TimePeriod): OverlayStyle {
  if (period.end_action.type === "fullscreen") {
    return (period.end_action as any).style || "semi_transparent";
  }
  return "semi_transparent";
}
function pad2(n: number): string {
  return n < 10 ? `0${n}` : `${n}`;
}
function clampMin(v: number, min: number, max: number): number {
  if (isNaN(v)) return min;
  return Math.max(min, Math.min(max, v));
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

    <!-- Tab 切换 -->
    <div class="tab-bar">
      <button :class="['tab', { active: activeTab === 'scheme' }]" @click="activeTab = 'scheme'">
        {{ t("settings.tabs.scheme") }}
      </button>
      <button :class="['tab', { active: activeTab === 'routine' }]" @click="activeTab = 'routine'">
        {{ t("settings.tabs.routine") }}
      </button>
      <button :class="['tab', { active: activeTab === 'weekly' }]" @click="activeTab = 'weekly'">
        {{ t("settings.tabs.weekly") }}
      </button>
      <button :class="['tab', { active: activeTab === 'system' }]" @click="activeTab = 'system'">
        {{ t("settings.tabs.system") }}
      </button>
    </div>

    <!-- =================== Tab 1: 方案管理 =================== -->
    <div v-show="activeTab === 'scheme'">
      <section>
        <h2>{{ t("settings.scheme.title") }}</h2>
        <p class="hint">{{ t("settings.scheme.hint") }}</p>
        <div class="scheme-toolbar">
          <select v-model="currentSchemeIndex" class="scheme-select">
            <option v-for="(s, i) in config.schemes" :key="s.id" :value="i">
              {{ s.name }}{{ isBuiltInScheme(s) ? " (" + t("settings.scheme.builtIn") + ")" : "" }}
            </option>
          </select>
          <button class="btn-sm" @click="onNewScheme">+ {{ t("settings.scheme.newScheme") }}</button>
          <button class="btn-sm" @click="onCloneScheme" :disabled="!currentScheme">⎘ {{ t("settings.scheme.cloneScheme") }}</button>
          <button class="btn-sm" @click="onRenameScheme" :disabled="!currentScheme || isBuiltInScheme(currentScheme!)">✎ 重命名</button>
          <button class="btn-sm danger" @click="onDeleteScheme" :disabled="!currentScheme || isBuiltInScheme(currentScheme!)">× {{ t("settings.scheme.deleteScheme") }}</button>
        </div>
      </section>

      <section v-if="currentScheme">
        <div class="section-head">
          <h2>{{ t("settings.stages") }} — {{ currentScheme.name }}</h2>
          <div class="actions">
            <button class="btn-sm" @click="addStage('work')">+ {{ t("settings.stageWork") }}</button>
            <button class="btn-sm" @click="addStage('rest')">+ {{ t("settings.stageRest") }}</button>
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
            <span class="stage-badge" :style="{ background: stageColor(s.type) }">{{ stageLabel(s.type) }}</span>
            <select v-model="s.type" class="stage-select">
              <option value="work">{{ t("settings.stageWork") }}</option>
              <option value="rest">{{ t("settings.stageRest") }}</option>
            </select>
            <input type="number" min="1" max="180" v-model.number="s.minutes" class="stage-min" />
            <span class="unit">{{ t("settings.minutes") }}</span>
            <div class="stage-ops">
              <button class="btn-icon" @click="moveUp(i)" :disabled="i === 0" title="上移">↑</button>
              <button class="btn-icon" @click="moveDown(i)" :disabled="i === currentScheme.stages.length - 1" title="下移">↓</button>
              <button class="btn-icon danger" @click="removeStage(i)" title="删除">×</button>
            </div>
          </div>
        </div>
        <p v-if="!hasWork" class="err">至少需要一个工作阶段</p>
      </section>

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

      <section v-if="currentScheme">
        <h2>{{ t("settings.sound") }}</h2>

        <div class="sound-section">
          <h3>{{ t("settings.workEndSound") }}</h3>
          <div class="sound-options">
            <label class="radio">
              <input type="radio" :checked="getSoundTypeValue(currentScheme.work_end_sound) === 'none'"
                @change="setSchemeSoundType('work_end_sound', 'none')" />
              {{ t("settings.soundNone") }}
            </label>
            <label class="radio">
              <input type="radio" :checked="getSoundTypeValue(currentScheme.work_end_sound) === 'builtin'"
                @change="setSchemeSoundType('work_end_sound', 'builtin')" />
              {{ t("settings.soundBuiltin") }}
            </label>
            <label class="radio" v-if="soundFiles.length > 0">
              <input type="radio" :checked="getSoundTypeValue(currentScheme.work_end_sound) !== 'none' && getSoundTypeValue(currentScheme.work_end_sound) !== 'builtin'"
                @change="(e) => { const v = (e.target as HTMLInputElement).value; }" />
              {{ t("settings.soundCustom") }}：
              <select :value="getSoundTypeValue(currentScheme.work_end_sound)"
                @change="(e) => setSchemeSoundType('work_end_sound', (e.target as HTMLSelectElement).value)">
                <option value="none">-- {{ t("settings.selectSoundFile") }} --</option>
                <option v-for="f in soundFiles" :key="f.file" :value="f.file">{{ f.name }}</option>
              </select>
            </label>
          </div>
        </div>

        <div class="sound-section">
          <h3>{{ t("settings.restEndSound") }}</h3>
          <div class="sound-options">
            <label class="radio">
              <input type="radio" :checked="getSoundTypeValue(currentScheme.rest_end_sound) === 'none'"
                @change="setSchemeSoundType('rest_end_sound', 'none')" />
              {{ t("settings.soundNone") }}
            </label>
            <label class="radio">
              <input type="radio" :checked="getSoundTypeValue(currentScheme.rest_end_sound) === 'builtin'"
                @change="setSchemeSoundType('rest_end_sound', 'builtin')" />
              {{ t("settings.soundBuiltin") }}
            </label>
            <label class="radio" v-if="soundFiles.length > 0">
              {{ t("settings.soundCustom") }}：
              <select :value="getSoundTypeValue(currentScheme.rest_end_sound)"
                @change="(e) => setSchemeSoundType('rest_end_sound', (e.target as HTMLSelectElement).value)">
                <option value="none">-- {{ t("settings.selectSoundFile") }} --</option>
                <option v-for="f in soundFiles" :key="f.file" :value="f.file">{{ f.name }}</option>
              </select>
            </label>
          </div>
        </div>

        <div class="sound-upload">
          <button class="btn-sm" @click="onUploadSound" :disabled="uploading">
            {{ uploading ? "上传中…" : "+ " + (t("settings.uploadSound") || "上传自定义音效") }}
          </button>
          <span class="hint">{{ t("settings.soundFileHint") }}</span>
        </div>

        <div v-if="soundFiles.length > 0" class="sound-list">
          <h3>{{ t("settings.uploadedSounds") }}</h3>
          <div v-for="f in soundFiles" :key="f.file" class="sound-item">
            <span>{{ f.name }}</span>
            <button class="btn-icon danger" @click="onDeleteSound(f.file)">{{ t("settings.deleteSound") }}</button>
          </div>
        </div>
      </section>
    </div>

    <!-- =================== Tab 2: 作息表 =================== -->
    <div v-show="activeTab === 'routine'">
      <section>
        <h2>{{ t("settings.routine.title") }}</h2>
        <p class="hint">{{ t("settings.routine.hint") }}</p>
        <div class="scheme-toolbar">
          <select v-model="currentRoutineIndex" class="scheme-select">
            <option v-for="(r, i) in config.routines" :key="r.id" :value="i">
              {{ r.name }}
            </option>
          </select>
          <button class="btn-sm" @click="onNewRoutine">+ {{ t("settings.routine.newRoutine") }}</button>
          <button class="btn-sm" @click="onRenameRoutine" :disabled="!currentRoutine">✎ 重命名</button>
          <button class="btn-sm danger" @click="onDeleteRoutine" :disabled="!currentRoutine">× {{ t("settings.routine.deleteRoutine") }}</button>
        </div>
      </section>

      <section v-if="currentRoutine">
        <div class="section-head">
          <h2>{{ currentRoutine.name }} — 时间段</h2>
          <div class="actions">
            <button class="btn-sm" @click="onNewPeriod" :disabled="config.schemes.length === 0">
              + {{ t("settings.routine.newPeriod") }}
            </button>
          </div>
        </div>

        <div v-if="currentRoutine.periods.length === 0" class="empty-hint">
          暂无时间段，点击「+ 添加时间段」开始配置。
        </div>

        <div v-else class="period-list">
          <div v-for="(p, i) in currentRoutine.periods" :key="p.id" class="period-card">
            <div class="period-row period-head">
              <span class="period-idx">#{{ i + 1 }}</span>
              <div class="period-ops">
                <button class="btn-icon" @click="onMovePeriodUp(i)" :disabled="i === 0" title="上移">↑</button>
                <button class="btn-icon" @click="onMovePeriodDown(i)" :disabled="i === currentRoutine.periods.length - 1" title="下移">↓</button>
                <button class="btn-icon danger" @click="onDeletePeriod(i)" title="删除">×</button>
              </div>
            </div>

            <div class="period-row">
              <div class="period-time">
                <label class="mini-label">{{ t("settings.routine.periodStart") }}</label>
                <div class="time-input">
              <input type="number" min="0" max="23"
                :value="p.start_hour"
                @change="(e) => p.start_hour = clampMin(parseInt((e.target as HTMLInputElement).value), 0, 23)" />
              <span>:</span>
              <input type="number" min="0" max="59"
                :value="p.start_minute"
                @change="(e) => p.start_minute = clampMin(parseInt((e.target as HTMLInputElement).value), 0, 59)" />
                </div>
              </div>

              <span class="time-arrow">→</span>

              <div class="period-time">
                <label class="mini-label">{{ t("settings.routine.periodEnd") }}</label>
                <div class="time-input">
              <input type="number" min="0" max="23"
                :value="p.end_hour"
                @change="(e) => p.end_hour = clampMin(parseInt((e.target as HTMLInputElement).value), 0, 23)" />
              <span>:</span>
              <input type="number" min="0" max="59"
                :value="p.end_minute"
                @change="(e) => p.end_minute = clampMin(parseInt((e.target as HTMLInputElement).value), 0, 59)" />
                </div>
              </div>

              <div class="period-scheme">
                <label class="mini-label">{{ t("settings.routine.periodScheme") }}</label>
                <select v-model="p.scheme_id">
                  <option v-for="s in config.schemes" :key="s.id" :value="s.id">{{ s.name }}</option>
                </select>
              </div>
            </div>

            <div class="period-row">
              <div class="period-end-action">
                <label class="mini-label">{{ t("settings.routine.periodEndAction") }}</label>
                <select :value="periodActionType(p)" @change="(e) => setEndActionType(p, ((e.target as HTMLSelectElement).value) as any)">
                  <option value="none">{{ t("settings.endAction.none") }}</option>
                  <option value="popup">{{ t("settings.endAction.popup") }}</option>
                  <option value="fullscreen">{{ t("settings.endAction.fullscreen") }}</option>
                  <option value="black_screen">{{ t("settings.endAction.black_screen") }}</option>
                </select>
              </div>
            </div>

            <div v-if="p.end_action.type !== 'none'" class="period-row end-action-detail">
              <div class="field">
                <label class="mini-label">{{ t("settings.endAction.text") }}</label>
                <input type="text"
                  :value="periodActionText(p)"
                  :placeholder="t('settings.endAction.textHint')"
                  @input="(e) => setEndActionText(p, (e.target as HTMLInputElement).value)" />
              </div>
              <div class="field">
                <label class="mini-label">{{ t("settings.endAction.sound") }}</label>
                <select :value="periodActionSound(p)"
                  @change="(e) => setEndActionSound(p, (e.target as HTMLSelectElement).value)">
                  <option value="none">{{ t("settings.soundNone") }}</option>
                  <option value="builtin">{{ t("settings.soundBuiltin") }}</option>
                  <option v-for="f in soundFiles" :key="f.file" :value="f.file">{{ f.name }}</option>
                </select>
              </div>
              <div class="field" v-if="p.end_action.type === 'fullscreen'">
                <label class="mini-label">{{ t("settings.endAction.style") }}</label>
                <select :value="periodActionStyle(p)"
                  @change="(e) => setEndActionStyle(p, (e.target as HTMLSelectElement).value as OverlayStyle)">
                  <option value="semi_transparent">{{ t("settings.styleSemiTransparent") }}</option>
                  <option value="full_black">{{ t("settings.styleFullBlack") }}</option>
                  <option value="dark">{{ t("settings.styleDark") }}</option>
                </select>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>

    <!-- =================== Tab 3: 周配置 =================== -->
    <div v-show="activeTab === 'weekly'">
      <section>
        <h2>{{ t("settings.weekly.title") }}</h2>
        <p class="hint">{{ t("settings.weekly.hint") }}</p>

        <table class="weekly-table">
          <thead>
            <tr>
              <th v-for="d in weekdayKeys" :key="d.key">{{ t("settings.weekly." + d.label) }}</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td v-for="d in weekdayKeys" :key="d.key">
                <select :value="config.weekly[d.key]" @change="(e) => onSetWeekday(d.key, (e.target as HTMLSelectElement).value)">
                  <option v-for="r in config.routines" :key="r.id" :value="r.id">{{ r.name }}</option>
                </select>
              </td>
            </tr>
          </tbody>
        </table>

        <div class="quick-actions">
          <button class="btn-sm" @click="() => {
            if (!config) return;
            const wd = config.routines[0]?.id;
            if (wd) {
              for (const k of ['monday','tuesday','wednesday','thursday','friday'] as const) {
                config.weekly[k] = wd;
              }
            }
          }">工作日统一</button>
          <button class="btn-sm" @click="() => {
            if (!config) return;
            const we = config.routines[config.routines.length - 1]?.id;
            if (we) {
              for (const k of ['saturday','sunday'] as const) {
                config.weekly[k] = we;
              }
            }
          }">周末统一</button>
        </div>
      </section>
    </div>

    <!-- =================== Tab 4: 系统设置 =================== -->
    <div v-show="activeTab === 'system'">
      <section>
        <h2>{{ t("settings.system") }}</h2>
        <div class="field">
          <label>{{ t("settings.resetThreshold") }}</label>
          <input type="number" min="1" v-model.number="config.reset_threshold_minutes" />
          <span class="unit">{{ t("settings.minutes") }}</span>
        </div>
        <div class="field">
          <label>{{ t("settings.notifyBefore") }}</label>
          <input type="number" min="0" max="30" v-model.number="config.notify_before_work_end_minutes" />
          <span class="unit">{{ t("settings.minutes") }}</span>
        </div>
      </section>

      <section>
        <h2>{{ t("settings.skipShortcut") }}</h2>
        <div class="field">
          <input type="text" v-model="config.skip_shortcut" placeholder="CmdOrCtrl+Shift+F2" />
          <span class="unit">Tauri accelerator 格式</span>
        </div>
        <p class="hint">用 CmdOrCtrl 跨平台自动映射</p>
      </section>

      <section v-if="!isMac">
        <h2>{{ t("settings.runAsAdmin") }}</h2>
        <label class="radio">
          <input type="checkbox" v-model="config.run_as_admin_autostart" />
          以管理员权限自启（提升遮罩覆盖能力，每次开机弹 UAC）
        </label>
        <p class="hint">默认关闭。开启后可覆盖任务管理器等管理员程序。</p>
      </section>

      <section>
        <h2>{{ t("settings.about") }}</h2>
        <div class="update-bar">
          <button class="btn-secondary" :disabled="checkingUpdate" @click="onCheckUpdate">
            {{ checkingUpdate ? "检查中…" : t("settings.checkUpdate") }}
          </button>
          <a href="https://github.com/weifeng-work/FocusLock/releases" target="_blank" class="update-link">
            {{ t("settings.githubRelease") }} →
          </a>
        </div>
        <div v-if="updateResult" :class="['msg', 'inline', updateResult.type]">{{ updateResult.text }}</div>
      </section>
    </div>

    <!-- 操作栏 -->
    <section class="actions-bar">
      <button class="btn-primary" :disabled="saving || !stagesValid" @click="onSave">
        {{ saving ? "保存中…" : t("settings.save") }}
      </button>
      <button class="btn-secondary" @click="onResetTimer">{{ t("settings.resetTimer") }}</button>
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
  max-width: 640px;
  margin: 0 auto;
  padding: 24px 20px 48px;
  color: #2c2c2a;
}
h1 {
  font-size: 22px;
  font-weight: 500;
  margin: 0 0 16px;
}
h2 {
  font-size: 15px;
  font-weight: 500;
  margin: 0 0 8px;
}
h3 {
  font-size: 13px;
  font-weight: 500;
  margin: 0 0 6px;
  color: #3a3935;
}
section {
  margin-bottom: 20px;
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
.empty-hint {
  padding: 20px;
  text-align: center;
  color: #888;
  font-size: 13px;
  background: #f7f6f2;
  border-radius: 6px;
}

/* Tab */
.tab-bar {
  display: flex;
  gap: 4px;
  margin-bottom: 20px;
  border-bottom: 1px solid #d3d1c7;
}
.tab {
  padding: 8px 16px;
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  font-size: 13px;
  color: #5f5e5a;
  cursor: pointer;
  transition: all 0.15s;
  margin-bottom: -1px;
}
.tab:hover {
  color: #2c2c2a;
}
.tab.active {
  color: #185fa5;
  border-bottom-color: #185fa5;
  font-weight: 500;
}

/* 头部 */
.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
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

/* 方案工具栏 */
.scheme-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.scheme-select {
  padding: 6px 10px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-size: 13px;
  min-width: 200px;
  flex: 1;
}

/* 阶段 */
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

/* 按钮 */
.btn-sm {
  padding: 5px 10px;
  font-size: 12px;
  border: 0.5px solid #888780;
  background: transparent;
  border-radius: 4px;
  cursor: pointer;
  white-space: nowrap;
}
.btn-sm:hover:not(:disabled) {
  background: #e6f1fb;
}
.btn-sm:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.btn-sm.danger:hover {
  background: #faece7;
  color: #993c1d;
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

/* 提示音 */
.sound-section {
  margin: 12px 0;
  padding: 10px;
  background: #f7f6f2;
  border-radius: 6px;
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
.sound-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0;
  font-size: 12px;
}

/* 字段 */
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

/* 时段卡片 */
.period-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.period-card {
  padding: 10px 12px;
  background: #f7f6f2;
  border-radius: 8px;
  border: 0.5px solid #e6e4d9;
}
.period-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  padding: 4px 0;
}
.period-head {
  justify-content: space-between;
  border-bottom: 0.5px solid #e6e4d9;
  margin-bottom: 4px;
  padding-bottom: 6px;
}
.period-idx {
  font-size: 12px;
  font-weight: 500;
  color: #185fa5;
}
.period-ops {
  display: flex;
  gap: 2px;
}
.mini-label {
  font-size: 11px;
  color: #888;
  margin-right: 4px;
}
.period-time {
  display: flex;
  align-items: center;
  gap: 4px;
}
.time-input {
  display: flex;
  align-items: center;
  gap: 2px;
}
.time-input input {
  width: 44px;
  padding: 4px 6px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-size: 13px;
  text-align: center;
}
.time-input span {
  font-size: 14px;
  font-weight: 500;
  color: #888;
}
.time-arrow {
  font-size: 16px;
  color: #888;
  margin: 0 8px;
}
.period-scheme {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 1;
  min-width: 200px;
}
.period-scheme select {
  flex: 1;
  padding: 4px 6px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-size: 13px;
}
.period-end-action {
  display: flex;
  align-items: center;
  gap: 4px;
  width: 100%;
}
.period-end-action select {
  flex: 1;
  max-width: 240px;
  padding: 4px 6px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-size: 13px;
}
.end-action-detail {
  padding: 8px 0 4px 12px;
  border-left: 2px solid #d3d1c7;
  margin-left: 4px;
}
.end-action-detail .field {
  margin: 4px 0;
}
.end-action-detail .field label {
  min-width: 70px;
}

/* 周配置 */
.weekly-table {
  width: 100%;
  border-collapse: collapse;
  margin: 12px 0;
}
.weekly-table th {
  text-align: left;
  padding: 6px 8px;
  font-size: 12px;
  color: #888;
  font-weight: 500;
  background: #f7f6f2;
  border-radius: 4px 4px 0 0;
}
.weekly-table td {
  padding: 6px 4px;
  border-bottom: 0.5px solid #e6e4d9;
}
.weekly-table select {
  width: 100%;
  padding: 4px 6px;
  border: 0.5px solid #b4b2a9;
  border-radius: 4px;
  font-size: 12px;
  background: #fff;
}
.quick-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

/* 操作栏 */
.actions-bar {
  display: flex;
  gap: 10px;
  border-bottom: none;
  position: sticky;
  bottom: 0;
  background: #fff;
  padding: 12px 0;
  border-top: 0.5px solid #d3d1c7;
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
.msg.ok { background: #e1f5ee; color: #085041; }
.msg.warn { background: #faeda; color: #633806; }
.msg.err { background: #faece7; color: #993c1d; }
.msg.info { background: #e8ecf4; color: #334e7a; }
.msg.inline { display: inline-block; margin-top: 8px; }

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
