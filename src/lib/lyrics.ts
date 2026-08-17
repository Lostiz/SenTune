export interface LyricLine {
  time: number;
  text: string;
  translation?: string;
}

const TIME_TAG_PATTERN = /\[(\d{1,2}):(\d{1,2})(?:[.:](\d{1,3}))?]/g;

/**
 * 解析 LRC 歌词文本为带时间戳的行。
 * 支持 `[mm:ss.xx]`、`[mm:ss:xx]`、一行多个时间标签。
 */
export function parseLrc(lrc: string | null | undefined): LyricLine[] {
  if (!lrc) return [];

  const lines: LyricLine[] = [];
  const rawLines = lrc.split(/\r?\n/);

  for (const raw of rawLines) {
    const matches = [...raw.matchAll(TIME_TAG_PATTERN)];
    if (matches.length === 0) continue;

    const text = raw.replace(TIME_TAG_PATTERN, "").trim();
    for (const match of matches) {
      const minutes = Number(match[1]);
      const seconds = Number(match[2]);
      const fractionRaw = match[3] ?? "";
      const fraction = fractionRaw
        ? Number(`0.${fractionRaw.padEnd(3, "0")}`)
        : 0;
      lines.push({
        time: minutes * 60 + seconds + fraction,
        text,
      });
    }
  }

  return lines.sort((a, b) => a.time - b.time);
}

/**
 * 合并原文歌词与翻译歌词，按时间点挂载翻译。
 */
export function mergeLyrics(
  lyric: string | null | undefined,
  translatedLyric: string | null | undefined,
): LyricLine[] {
  const original = parseLrc(lyric);
  const translation = parseLrc(translatedLyric);

  const translationByTime = new Map<number, string>();
  for (const line of translation) {
    if (line.text && !translationByTime.has(line.time)) {
      translationByTime.set(line.time, line.text);
    }
  }

  return original.map((line) => ({
    ...line,
    translation: translationByTime.get(line.time),
  }));
}

/**
 * 根据当前播放时间返回当前应高亮的歌词行索引；没有匹配时返回 -1。
 */
export function findActiveLyricIndex(
  lines: LyricLine[],
  currentTime: number,
): number {
  if (lines.length === 0) return -1;

  let activeIndex = -1;
  for (let index = 0; index < lines.length; index += 1) {
    if (currentTime >= lines[index].time) {
      activeIndex = index;
    } else {
      break;
    }
  }

  return activeIndex;
}
