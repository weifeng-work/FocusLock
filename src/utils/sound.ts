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

// 播放内置提示音（根据变体名称）
// soundName: 音效变体名称（alarm/chime/digital/pulse/bird）
// duration: 持续时间（秒），默认 10 秒
// volume: 音量 (0-1)，默认 0.7 (70%)
export function playBuiltinSound(soundName: string = "alarm", duration: number = 10, volume: number = 0.7): void {
  try {
    const ctx = getAudioContext();
    const now = ctx.currentTime;
    
    // 根据音效变体名称调用对应的生成函数
    switch (soundName) {
      case "chime":
        playChimeSound(ctx, now, duration, volume);
        break;
      case "digital":
        playDigitalSound(ctx, now, duration, volume);
        break;
      case "pulse":
        playPulseSound(ctx, now, duration, volume);
        break;
      case "bird":
        playBirdSound(ctx, now, duration, volume);
        break;
      case "alarm":
      default:
        playAlarmSound(ctx, now, duration, volume);
        break;
    }
  } catch (e) {
    console.warn("播放内置提示音失败:", e);
  }
}

// 闹钟式提示音：800Hz 和 600Hz 交替，每秒 2 次
function playAlarmSound(ctx: AudioContext, now: number, duration: number, volume: number): void {
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
}

// 风铃声：523/659/784Hz 三音叠加，指数衰减
function playChimeSound(ctx: AudioContext, now: number, duration: number, volume: number): void {
  const masterGain = ctx.createGain();
  masterGain.gain.value = volume;
  masterGain.connect(ctx.destination);
  
  // 三个频率：C5(523Hz), E5(659Hz), G5(784Hz)
  const freqs = [523, 659, 784];
  
  for (let i = 0; i < freqs.length; i++) {
    const oscillator = ctx.createOscillator();
    const gainNode = ctx.createGain();
    
    oscillator.connect(gainNode);
    gainNode.connect(masterGain);
    
    oscillator.frequency.value = freqs[i];
    oscillator.type = "sine";
    
    // 指数衰减：快速淡入，缓慢淡出
    const startTime = now + i * 0.1; // 稍微错开开始时间，产生琶音效果
    gainNode.gain.setValueAtTime(0, startTime);
    gainNode.gain.linearRampToValueAtTime(0.6, startTime + 0.05);
    gainNode.gain.exponentialRampToValueAtTime(0.01, startTime + duration * 0.8);
    
    oscillator.start(startTime);
    oscillator.stop(startTime + duration * 0.8);
  }
  
  // duration 后停止所有声音
  setTimeout(() => {
    masterGain.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.1);
    setTimeout(() => {
      masterGain.disconnect();
    }, 200);
  }, duration * 1000);
}

// 电子哔声：800Hz→1000Hz 短促双音重复
function playDigitalSound(ctx: AudioContext, now: number, duration: number, volume: number): void {
  const masterGain = ctx.createGain();
  masterGain.gain.value = volume;
  masterGain.connect(ctx.destination);
  
  const beepDuration = 0.08; // 每次蜂鸣持续 0.08 秒（短促）
  const beepInterval = 0.3; // 每次蜂鸣间隔 0.3 秒
  const totalBeeps = Math.floor(duration / beepInterval);
  
  for (let i = 0; i < totalBeeps; i++) {
    const beepTime = now + i * beepInterval;
    
    // 第一个音：800Hz
    const osc1 = ctx.createOscillator();
    const gain1 = ctx.createGain();
    
    osc1.connect(gain1);
    gain1.connect(masterGain);
    
    osc1.frequency.value = 800;
    osc1.type = "square"; // 方波，电子感
    
    gain1.gain.setValueAtTime(0, beepTime);
    gain1.gain.linearRampToValueAtTime(1, beepTime + 0.01);
    gain1.gain.setValueAtTime(1, beepTime + 0.04);
    gain1.gain.linearRampToValueAtTime(0, beepTime + beepDuration);
    
    osc1.start(beepTime);
    osc1.stop(beepTime + beepDuration);
    
    // 第二个音：1000Hz（稍微延迟）
    const osc2 = ctx.createOscillator();
    const gain2 = ctx.createGain();
    
    osc2.connect(gain2);
    gain2.connect(masterGain);
    
    osc2.frequency.value = 1000;
    osc2.type = "square";
    
    const beepTime2 = beepTime + 0.1;
    gain2.gain.setValueAtTime(0, beepTime2);
    gain2.gain.linearRampToValueAtTime(1, beepTime2 + 0.01);
    gain2.gain.setValueAtTime(1, beepTime2 + 0.04);
    gain2.gain.linearRampToValueAtTime(0, beepTime2 + beepDuration);
    
    osc2.start(beepTime2);
    osc2.stop(beepTime2 + beepDuration);
  }
  
  // duration 后停止所有声音
  setTimeout(() => {
    masterGain.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.1);
    setTimeout(() => {
      masterGain.disconnect();
    }, 200);
  }, duration * 1000);
}

// 柔和脉冲：400Hz 正弦波，0.5s 开/0.5s 关
function playPulseSound(ctx: AudioContext, now: number, duration: number, volume: number): void {
  const masterGain = ctx.createGain();
  masterGain.gain.value = volume;
  masterGain.connect(ctx.destination);
  
  const pulseDuration = 0.5; // 每次脉冲持续 0.5 秒
  const pulseInterval = 1.0; // 每次脉冲间隔 1.0 秒（0.5s 开 + 0.5s 关）
  const totalPulses = Math.floor(duration / pulseInterval);
  
  for (let i = 0; i < totalPulses; i++) {
    const pulseTime = now + i * pulseInterval;
    
    const oscillator = ctx.createOscillator();
    const gainNode = ctx.createGain();
    
    oscillator.connect(gainNode);
    gainNode.connect(masterGain);
    
    oscillator.frequency.value = 400; // 柔和的中低频
    oscillator.type = "sine"; // 正弦波，最柔和
    
    // 音量包络：淡入 → 保持 → 淡出
    gainNode.gain.setValueAtTime(0, pulseTime);
    gainNode.gain.linearRampToValueAtTime(1, pulseTime + 0.05);
    gainNode.gain.setValueAtTime(1, pulseTime + pulseDuration - 0.05);
    gainNode.gain.linearRampToValueAtTime(0, pulseTime + pulseDuration);
    
    oscillator.start(pulseTime);
    oscillator.stop(pulseTime + pulseDuration);
  }
  
  // duration 后停止所有声音
  setTimeout(() => {
    masterGain.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.1);
    setTimeout(() => {
      masterGain.disconnect();
    }, 200);
  }, duration * 1000);
}

// 鸟鸣声：频率调制的短促啁啾声
function playBirdSound(ctx: AudioContext, now: number, duration: number, volume: number): void {
  const masterGain = ctx.createGain();
  masterGain.gain.value = volume;
  masterGain.connect(ctx.destination);
  
  // 模拟鸟鸣：多个短促的频率调制音
  const chirpCount = Math.floor(duration / 0.8); // 每 0.8 秒一个啁啾
  
  for (let i = 0; i < chirpCount; i++) {
    const chirpTime = now + i * 0.8;
    const baseFreq = 2000 + Math.random() * 1000; // 随机基频 2000-3000Hz
    
    // 创建频率调制的啁啾声
    const oscillator = ctx.createOscillator();
    const gainNode = ctx.createGain();
    
    oscillator.connect(gainNode);
    gainNode.connect(masterGain);
    
    oscillator.type = "sine";
    
    // 频率调制：快速上升然后下降
    oscillator.frequency.setValueAtTime(baseFreq, chirpTime);
    oscillator.frequency.linearRampToValueAtTime(baseFreq * 1.5, chirpTime + 0.05);
    oscillator.frequency.linearRampToValueAtTime(baseFreq * 0.8, chirpTime + 0.15);
    
    // 音量包络：快速淡入淡出
    gainNode.gain.setValueAtTime(0, chirpTime);
    gainNode.gain.linearRampToValueAtTime(0.8, chirpTime + 0.02);
    gainNode.gain.setValueAtTime(0.8, chirpTime + 0.1);
    gainNode.gain.linearRampToValueAtTime(0, chirpTime + 0.15);
    
    oscillator.start(chirpTime);
    oscillator.stop(chirpTime + 0.15);
    
    // 有时添加第二个啁啾（模拟鸟叫的重复）
    if (Math.random() > 0.5) {
      const chirpTime2 = chirpTime + 0.2;
      const oscillator2 = ctx.createOscillator();
      const gainNode2 = ctx.createGain();
      
      oscillator2.connect(gainNode2);
      gainNode2.connect(masterGain);
      
      oscillator2.type = "sine";
      oscillator2.frequency.setValueAtTime(baseFreq * 1.2, chirpTime2);
      oscillator2.frequency.linearRampToValueAtTime(baseFreq * 0.9, chirpTime2 + 0.1);
      
      gainNode2.gain.setValueAtTime(0, chirpTime2);
      gainNode2.gain.linearRampToValueAtTime(0.6, chirpTime2 + 0.02);
      gainNode2.gain.linearRampToValueAtTime(0, chirpTime2 + 0.1);
      
      oscillator2.start(chirpTime2);
      oscillator2.stop(chirpTime2 + 0.1);
    }
  }
  
  // duration 后停止所有声音
  setTimeout(() => {
    masterGain.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.1);
    setTimeout(() => {
      masterGain.disconnect();
    }, 200);
  }, duration * 1000);
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
// soundType 格式: "none" / "builtin:alarm" / "custom:文件名"
// duration: 持续时间（秒），默认 10 秒
// volume: 音量 (0-1)，默认 0.7 (70%)
export async function playSound(soundType: string, duration: number = 10, volume: number = 0.7): Promise<void> {
  if (soundType === "none") return;

  if (soundType.startsWith("builtin:")) {
    const soundName = soundType.substring(8); // 移除 "builtin:" 前缀
    playBuiltinSound(soundName, duration, volume);
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
