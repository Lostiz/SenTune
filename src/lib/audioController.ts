/**
 * 全局音频控制器：播放/暂停/seek 只由用户意图驱动，
 * 避免 React 状态竞态导致 play() 被 pause() 打断。
 */
let audio: HTMLAudioElement | null = null;

export function bindAudio(element: HTMLAudioElement | null) {
  audio = element;
}

export function playAudio(): Promise<void> | undefined {
  if (!audio) return undefined;
  return audio.play();
}

export function pauseAudio() {
  audio?.pause();
}

export function seekAudio(time: number) {
  if (audio && Number.isFinite(time) && time >= 0) {
    audio.currentTime = time;
  }
}

export function setAudioVolume(volume: number) {
  if (audio) {
    audio.volume = volume;
  }
}
