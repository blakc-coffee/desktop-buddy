import { invoke, isTauri } from "@tauri-apps/api/core";
import { emptyItem, type Item, type Snapshot, validateBackup } from "./domain";
export const native = isTauri();
const key = "desktop-buddy-preview-v1";
const read = (): Snapshot => {
  const raw = localStorage.getItem(key);
  return {
    items: raw ? JSON.parse(raw).items : [],
    theme: raw ? JSON.parse(raw).theme : "dark",
    dataPath: "Browser preview storage · separate from the desktop database",
    warning: null,
    petAppearance: "calico",
    quietMode: raw ? !!JSON.parse(raw).quietMode : false,
    native: false,
    agentHistory: raw ? JSON.parse(raw).agentHistory || [] : [],
  };
};
const write = (s: Snapshot) => {
  localStorage.setItem(key, JSON.stringify(s));
  window.dispatchEvent(new Event("preview-changed"));
};
export async function call<T = void>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (native) return invoke<T>(command, args);
  const s = read();
  const id = args.id as string;
  const item = s.items.find((i) => i.id === id);
  switch (command) {
    case "rename_todo": {
      if (!item || item.kind !== "todo")
        throw new Error("This item is no longer a todo.");
      const title = String(args.title || "").trim();
      if (!title || title.length > 240)
        throw new Error("Use a short todo title.");
      item.title = title;
      break;
    }
    case "snapshot":
      return s as T;
    case "save_item": {
      const i = args.item as Item;
      if (!i.title.trim() || i.title.length > 240)
        throw new Error("Use a title between 1 and 240 characters.");
      const old = s.items.findIndex((x) => x.id === i.id);
      if (old < 0) s.items.unshift({ ...i, id: crypto.randomUUID() });
      else s.items[old] = i;
      break;
    }
    case "toggle_item":
      if (item) item.done = !item.done;
      break;
    case "remove_item":
      s.items = s.items.filter((i) => i.id !== id);
      break;
    case "start_timer": {
      const duration = Number(args.minutes) * 60000;
      s.items.unshift({
        ...emptyItem("timer", String(args.title) || "Focus session"),
        id: crypto.randomUUID(),
        state: "running",
        durationMs: duration,
        remainingMs: duration,
        dueAt: Date.now() + duration,
        projectId: String(args.projectId || ""),
      });
      break;
    }
    case "timer_action":
      if (item) {
        if (args.action === "pause") {
          item.remainingMs = Math.max(0, item.dueAt - Date.now());
          item.state = "paused";
        } else if (args.action === "resume") {
          item.dueAt = Date.now() + item.remainingMs;
          item.state = "running";
        } else {
          item.remainingMs = item.durationMs;
          item.state = "paused";
        }
      }
      break;
    case "set_quiet_mode":
      s.quietMode = Boolean(args.enabled);
      break;
    case "set_theme":
      s.theme = String(args.theme);
      break;
    case "export_backup":
      return JSON.stringify(
        { version: 2, items: s.items, agentTasks: s.agentHistory || [] },
        null,
        2,
      ) as T;
    case "import_backup": {
      const data = validateBackup(String(args.data));
      const history = JSON.parse(String(args.data)).agentTasks;
      if (Array.isArray(history))
        s.agentHistory = [
          ...(s.agentHistory || []),
          ...history.filter(
            (i) =>
              !(s.agentHistory || []).some(
                (e) => (e as { id?: string }).id === i.id,
              ),
          ),
        ];
      const missing = data.filter((i) => !s.items.some((e) => e.id === i.id));
      s.items.push(
        ...missing.map((i) =>
          i.kind === "timer" && i.state === "running"
            ? { ...i, state: "paused" }
            : i,
        ),
      );
      write(s);
      return missing.length as T;
    }
    case "providers":
      return [
        {
          id: "codex",
          installed: false,
          path: "",
          description: "Open the desktop app to detect the Codex CLI.",
        },
        {
          id: "agy",
          installed: false,
          path: "",
          description: "Antigravity ACP is not connected in this milestone.",
        },
      ] as T;
    default:
      throw new Error("This action needs the native desktop app.");
  }
  write(s);
  return undefined as T;
}
export function previewReconcile() {
  if (native) return;
  const s = read();
  let dirty = false;
  for (const i of [...s.items])
    if (
      ((i.kind === "timer" && i.state === "running") ||
        (i.kind === "reminder" && !i.done)) &&
      i.dueAt <= Date.now()
    ) {
      if (i.kind === "timer") {
        i.state = "finished";
        i.remainingMs = 0;
      } else i.done = true;
      s.items.unshift({
        ...emptyItem("attention", i.title),
        id: `due-${i.id}`,
        source: i.kind === "timer" ? "Timer finished" : "Reminder due",
      });
      dirty = true;
    }
  if (dirty) write(s);
}
