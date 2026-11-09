import { invoke } from "@tauri-apps/api/core";
import type { AppStatus, Settings, SyncResult, Task } from "./types";

export const backend = {
  status: () => invoke<AppStatus>("get_app_status"),
  listTasks: () => invoke<Task[]>("list_tasks"),
  createTask: (title: string) => invoke<Task>("create_task", { title }),
  saveTask: (task: Task) => invoke<Task>("save_task", { input: task }),
  completeTask: (id: string, done: boolean) =>
    invoke<Task>("complete_task", { id, done }),
  sync: () => invoke<SyncResult>("sync_now"),
  connect: (token: string, settings: Settings) =>
    invoke<AppStatus>("connect_notion", { input: { token, settings } }),
  saveSettings: (settings: Settings) =>
    invoke<AppStatus>("save_settings", { settings }),
  disconnect: () => invoke<AppStatus>("disconnect_notion"),
};
