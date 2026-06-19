// 音效播放工具
// 支持：内置提示音（Web Audio API 生成）、自定义音频文件（HTML5 Audio API）

import { invoke } from "@tauri-apps/api/core";

// AudioContext 单例
let audioContext: AudioContext | null = null;

function getAudioContext(): AudioContext {
  if (!audioContext) {
    audioContext = new AudioContext();
  }
  return audioContext;
}

// 播放内置提示音（简单的"哔"声）
export function playBuiltinSound(): void {
  try {
    const ctx = getAudioContext();
    const oscillator = ctx.createOscillator();
    const gainNode = ctx.createGain();

    oscillator.connect(gainNode);
    gainNode.connect(ctx.destination);

    // 设置音调和音量
    oscillator.frequency.value = 800; // 800Hz
    oscillator.type = "sine"; // 正弦波，声音较柔和

    gainNode.gain.setValueAtTime(0.3, ctx.currentTime); // 音量 30%
    gainNode.gain.exponentialRampToValueAtTime(0.01, ctx.currentTime + 0.5); // 淡出

    oscillator.start(ctx.currentTime);
    oscillator.stop(ctx.currentTime + 0.5); // 播放 0.5 秒
  } catch (e) {
    console.warn("播放内置提示音失败:", e);
  }
}

// 播放自定义音频文件
export async function playCustomSound(fileName: string): Promise<void> {
  try {
    // 让后端读取音频文件并返回 base64 编码
    const { invoke } = await import("@tauri-apps/api/core");
    const base64Data = await invoke<string>("read_sound_file", { fileName });
    
    // 将 base64 转换为 blob URL
    const binaryString = atob(base64Data);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    const blob = new Blob([bytes], { type: "audio/*" });
    const audioUrl = URL.createObjectURL(blob);

    const audio = new Audio(audioUrl);
    audio.volume = 0.7; // 音量 70%
    await audio.play();

    // 播放完毕后释放 blob URL
    audio.onended = () => {
      URL.revokeObjectURL(audioUrl);
    };
  } catch (e) {
    console.warn("播放自定义音效失败:", e);
  }
}

// 播放音效（根据配置）
export async function playSound(soundType: string): Promise<void> {
  if (soundType === "none") return;

  if (soundType === "builtin") {
    playBuiltinSound();
    return;
  }

  // 自定义音频：soundType 格式为 "custom:文件名"
  if (soundType.startsWith("custom:")) {
    const fileName = soundType.substring(7);
    await playCustomSound(fileName);
    return;
  }

  console.warn("未知的音效类型:", soundType);
}

// 解析并播放音效事件
// eventData 格式: "work_end:builtin" 或 "rest_end:custom:文件名"
export async function handlePlaySoundEvent(eventData: string): Promise<void> {
  // eventData 格式: "work_end:builtin" 或 "rest_end:custom:文件名"
  const parts = eventData.split(":");
  if (parts.length < 2) return;

  const soundValue = parts.slice(1).join(":"); // 支持文件名中包含 ":"
  await playSound(soundValue);
}
