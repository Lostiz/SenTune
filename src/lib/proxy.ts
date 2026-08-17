import { invoke } from "@tauri-apps/api/core";

let portPromise: Promise<number> | null = null;

export function getProxyPort(): Promise<number> {
  if (!portPromise) {
    portPromise = invoke<number>("get_proxy_port").catch(() => 0);
  }
  return portPromise;
}

/** 将 B 站封面地址转为本地代理地址（带 UA/Referer，解决防盗链）。 */
export async function coverProxyUrl(src: string): Promise<string> {
  if (!src) return "";
  const port = await getProxyPort();
  if (!port) return src;
  return `http://127.0.0.1:${port}/cover/remote?url=${encodeURIComponent(src)}`;
}
