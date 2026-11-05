export interface Item {
  id: string;
  kind:
    | "todo"
    | "reminder"
    | "timer"
    | "project"
    | "shelf"
    | "attention"
    | "receipt";
  title: string;
  done: boolean;
  createdAt: number;
  dueAt: number;
  path: string;
  note: string;
  source: string;
  projectId: string;
  state: string;
  durationMs: number;
  remainingMs: number;
  startedTick: number;
  run: string;
}
export interface Snapshot {
  items: Item[];
  theme: string;
  dataPath: string;
  warning: string | null;
  native: boolean;
  agentAttention?: number;
  agentHistory?: unknown[];
}
export const emptyItem = (kind: Item["kind"], title: string): Item => ({
  id: "",
  kind,
  title,
  done: false,
  createdAt: Date.now(),
  dueAt: 0,
  path: "",
  note: "",
  source: "",
  projectId: "",
  state: "",
  durationMs: 0,
  remainingMs: 0,
  startedTick: 0,
  run: "",
});
export const formatDuration = (ms: number) => {
  const s = Math.max(0, Math.ceil(ms / 1000));
  return `${Math.floor(s / 60)
    .toString()
    .padStart(2, "0")}:${(s % 60).toString().padStart(2, "0")}`;
};
export type Capture =
  | { type: "todo"; title: string }
  | { type: "timer"; title: string; minutes: number }
  | { type: "reminder"; title: string; dueAt: number }
  | { type: "error"; message: string };
export function parseCapture(input: string, now = new Date()): Capture {
  const text = input.trim();
  if (!text) return { type: "error", message: "Enter something to capture." };
  const timer = /^timer\s+(\d{1,4})m(?:\s+(.+))?$/i.exec(text);
  if (timer) {
    const minutes = Number(timer[1]);
    return minutes > 0 && minutes <= 1440
      ? { type: "timer", title: timer[2] || "Focus session", minutes }
      : { type: "error", message: "Choose 1–1440 minutes." };
  }
  const remind = /^remind\s+(today|tomorrow)\s+(\d{1,2}):(\d{2})\s+(.+)$/i.exec(
    text,
  );
  if (remind) {
    const h = Number(remind[2]),
      m = Number(remind[3]);
    if (h > 23 || m > 59)
      return { type: "error", message: "Use a 24-hour time, such as 16:30." };
    const d = new Date(now);
    if (remind[1].toLowerCase() === "tomorrow") d.setDate(d.getDate() + 1);
    d.setHours(h, m, 0, 0);
    return d.getTime() <= now.getTime()
      ? {
          type: "error",
          message: "That time has passed. Choose a future time.",
        }
      : { type: "reminder", title: remind[4], dueAt: d.getTime() };
  }
  if (/^timer\b/i.test(text))
    return {
      type: "error",
      message: "Use timer 25m, optionally followed by a task.",
    };
  if (/^remind\b/i.test(text))
    return {
      type: "error",
      message: "Use remind tomorrow 10:00 review tests (24-hour time).",
    };
  return { type: "todo", title: text.replace(/^todo\s+/i, "") };
}
export function validateBackup(text: string): Item[] {
  if (text.length > 5 * 1024 * 1024) throw new Error("Backup exceeds 5 MiB.");
  const data = JSON.parse(text);
  if (
    ![1, 2].includes(data.version) ||
    !Array.isArray(data.items) ||
    data.items.length > 2000
  )
    throw new Error("This is not a supported Buddy backup.");
  for (const i of data.items)
    if (
      !i ||
      typeof i.id !== "string" ||
      !i.id ||
      typeof i.title !== "string" ||
      !i.title.trim() ||
      i.title.length > 240 ||
      !Number.isFinite(i.createdAt) ||
      i.createdAt < 0 ||
      i.createdAt > 8640000000000000 ||
      !Number.isFinite(i.dueAt) ||
      i.dueAt < 0 ||
      i.dueAt > 8640000000000000 ||
      ![
        "todo",
        "reminder",
        "timer",
        "project",
        "shelf",
        "attention",
        "receipt",
      ].includes(i.kind) ||
      typeof i.note !== "string" ||
      i.note.length > 65536
    )
      throw new Error("Backup contains invalid items.");
  return data.items;
}
