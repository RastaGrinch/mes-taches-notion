export type Priority =
  | "P1 - Urgent & Important"
  | "P2 - Important"
  | "P3 - A faire"
  | "P4 - Plus tard"
  | "";

export interface Task {
  id: string;
  notionId: string | null;
  title: string;
  done: boolean;
  status: string;
  priority: Priority;
  due: string | null;
  completedAt: string | null;
  assignee: string;
  source: string;
  taskType: string;
  description: string;
  comment: string;
  notionUrl: string | null;
  dirty: boolean;
  syncError: string | null;
  updatedAt: string;
}

export interface Settings {
  dataSourceId: string;
  defaultStatus: string;
  defaultPriority: Priority;
  defaultAssignee: string;
  defaultSource: string;
  syncIntervalMinutes: number;
}

export interface AppStatus {
  configured: boolean;
  tokenStored: boolean;
  settings: Settings;
  pendingCount: number;
}

export interface SyncResult {
  pushed: number;
  pulled: number;
  failed: number;
  pending: number;
  message: string;
}

export type ViewKey = "today" | "todo" | "important" | "later" | "completed";
