import { listen } from "@tauri-apps/api/event";
import type { Change, PollResult } from "./types";

export function onPollUpdate(callback: (result: PollResult) => void) {
  return listen<PollResult>("poll-update", (event) => callback(event.payload));
}

export function onChange(callback: (change: Change) => void) {
  return listen<Change>("pr-change", (event) => callback(event.payload));
}
