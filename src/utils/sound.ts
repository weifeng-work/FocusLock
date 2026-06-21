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

// 播放内置提示音（闹钟式"滴滴答"声）
// duration: 持续时间（秒），默认 10 秒
// volume: 音量 (0-1)，默认 0.7 (70%)
export function playBuiltinSound(duration: number = 10, volume: number = 0.7): void {
  try {
    const ctx = getAudioContext();
    const now = ctx.currentTime;
    
    // 创建主增益节点（控制总音量）
    const masterGain = ctx.createGain();
    masterGain.gain.value = volume;
    masterGain.connect(ctx.destination);
    
    // 闹钟式提示音：800Hz 和 600Hz 交替，每秒 2 次
    const beepDuration = 0.2; // 每次蜂鸣持续 0.2 秒
    const beepInterval = 0.5; // 每次蜂鸣间隔 0.5 秒（包含 0.2 秒蜂鸣 + 0.3 秒静音）
    const totalBeeps = Math.floor(duration / beepInterval);
    
    for (let i = 0; i < totalBeeps; i++) {
      const beepTime = now + i * beepInterval;
      const freq = i % 2 === 0 ? 800 : 600; // 交替频率
      
      const oscillator = ctx.createOscillator();
      const gainNode = ctx.createGain();
      
      oscillator.connect(gainNode);
      gainNode.connect(masterGain);
      
      oscillator.frequency.value = freq;
      oscillator.type = "sine";
      
      // 音量包络：快速淡入 + 淡出（避免爆音）
      gainNode.gain.setValueAtTime(0, beepTime);
      gainNode.gain.linearRampToValueAtTime(1, beepTime + 0.02);
      gainNode.gain.setValueAtTime(1, beepTime + beepDuration - 0.02);
      gainNode.gain.linearRampToValueAtTime(0, beepTime + beepDuration);
      
      oscillator.start(beepTime);
      oscillator.stop(beepTime + beepDuration);
    }
    
    // duration 后停止所有声音
    setTimeout(() => {
      masterGain.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.1);
      setTimeout(() => {
        masterGain.disconnect();
      }, 200);
    }, duration * 1000);
    
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
// duration: 持续时间（秒），默认 10 秒
// volume: 音量 (0-1)，默认 0.7 (70%)
export async function playSound(soundType: string, duration: number = 10, volume: number = 0.7): Promise<void> {
  if (soundType === "none") return;

  if (soundType === "builtin") {
    playBuiltinSound(duration, volume);
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
// 使用默认参数：duration=10秒, volume=0.7 (70%)
export async function handlePlaySoundEvent(eventData: string): Promise<void> {
  // eventData 格式: "work_end:builtin" 或 "rest_end:custom:文件名"
  const parts = eventData.split(":");
  if (parts.length < 2) return;

  const soundValue = parts.slice(1).join(":"); // 支持文件名中包含 ":"
  await playSound(soundValue, 10, 0.7);
}
