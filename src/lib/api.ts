import { invoke } from "@tauri-apps/api/core";
import type { Config, PollResult, PrDetail, PrId } from "./types";

export const api = {
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (config: Config) => invoke<void>("save_config", { config }),
  storeToken: (providerName: string, token: string) =>
    invoke<void>("store_token", { providerName, token }),
  deleteToken: (providerName: string) =>
    invoke<void>("delete_token", { providerName }),
  testConnection: (providerName: string, url: string, token: string) =>
    invoke<string>("test_connection", { providerName, url, token }),
  initProviders: () => invoke<string[]>("init_providers"),
  pollAll: () => invoke<PollResult>("poll_all"),
  getPrDetail: (prId: PrId) => invoke<PrDetail>("get_pr_detail", { prId }),
  listProjects: (providerName: string, url: string, token: string) =>
    invoke<string[]>("list_projects", { providerName, url, token }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  startPolling: () => invoke<void>("start_polling"),
};
