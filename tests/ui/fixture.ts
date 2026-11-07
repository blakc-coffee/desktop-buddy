import type { Page } from "@playwright/test";
export async function installFixture(
  page: Page,
  theme = "dark",
  empty = false,
) {
  const initialize = ({ theme, empty }: { theme: string; empty: boolean }) => {
    const root = window as unknown as Record<string, any>;
    const now = Date.now();
    const item = (id: string, kind: string, title: string, extra = {}) => ({
      id,
      kind,
      title,
      done: false,
      createdAt: now,
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
      ...extra,
    });
    let items = empty
      ? []
      : [
          item("p1", "project", "Desktop Buddy", {
            path: "C:\\Projects\\Desktop Buddy",
            note: "Review the file handoff, then polish the release checklist.",
          }),
          item("p2", "project", "Algorithms practice", {
            path: "C:\\Projects\\University\\Semester 5\\Algorithms and data structures",
            note: "Finish the sliding-window problem and write up its complexity.",
          }),
          item("t1", "todo", "Review the browser-helper setup", {
            projectId: "p1",
          }),
          item("t2", "todo", "Write the complexity analysis", {
            projectId: "p2",
          }),
          item("t3", "todo", "Save the project notes", { done: true }),
          item("focus", "timer", "Algorithms practice", {
            state: "paused",
            durationMs: 1500000,
            remainingMs: 1230000,
          }),
          item("r1", "reminder", "Review the release notes", {
            dueAt: now + 3600000,
          }),
          item("a1", "attention", "Your focus session finished", {
            source: "Timer finished",
          }),
          item("s1", "shelf", "Implementation notes", {
            note: "Check drag formats and explain the supported browser editors.",
            projectId: "p1",
          }),
          item("s2", "shelf", "Problem statement.pdf", {
            path: "C:\\Projects\\University\\Semester 5\\Reference material\\Problem statement and assessment requirements.pdf",
            projectId: "p2",
          }),
        ];
    const callbacks: Record<number, (event: unknown) => void> = {};
    const listeners: Record<number, { event: string; handler: number }> = {};
    let next = 1;
    root.isTauri = true;
    root.fixtureCalls = [];
    root.fixtureAcks = [];
    root.fixtureTasks = [];
    root.fixturePending = null;
    root.fixtureEmit = (event: string, payload: unknown) => {
      if (event === "panel-navigation") root.fixturePending = payload;
      for (const listener of Object.values(listeners))
        if (listener.event === event)
          callbacks[listener.handler]?.({ payload });
    };
    root.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    root.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { label: "main" },
      },
      transformCallback: (callback: (event: unknown) => void) => {
        const id = next++;
        callbacks[id] = callback;
        return id;
      },
      unregisterCallback: (id: number) => {
        delete callbacks[id];
      },
      invoke: async (command: string, args: Record<string, any> = {}) => {
        root.fixtureCalls.push({ command, args });
        if (command === "plugin:event|listen") {
          const id = next++;
          listeners[id] = args as { event: string; handler: number };
          return id;
        }
        if (command === "plugin:event|unlisten") {
          delete listeners[args.eventId];
          return;
        }
        if (command === "snapshot" && root.fixtureSnapshotError) {
          root.fixtureSnapshotError--;
          throw Error("Synthetic storage temporarily unavailable");
        }
        if (command === "snapshot")
          return {
            items,
            theme,
            dataPath: "Synthetic UI fixture storage",
            warning: null,
            native: true,
            agentAttention: root.fixtureTasks.filter(
              (task: any) => task.pending.length > 0,
            ).length,
            petAppearance: "calico",
          };
        if (command === "providers")
          return [
            {
              id: "codex",
              installed: true,
              path: "fixture",
              description:
                "Synthetic detected CLI. No model requests are sent.",
            },
            {
              id: "agy",
              installed: false,
              path: "",
              description: "Antigravity connection is not available.",
            },
          ];
        if (command === "panel_navigation") return root.fixturePending;
        if (command === "acknowledge_navigation") {
          root.fixtureAcks.push(args.id);
          if (root.fixturePending?.id === args.id) root.fixturePending = null;
          return;
        }
        if (command === "agent_snapshot") return root.fixtureTasks;
        if (command === "browser_connections") return [];
        if (command === "codex_sessions") return { data: [] };
        if (command === "set_theme") {
          theme = args.theme;
          root.fixtureEmit("data-changed", null);
          return;
        }
        if (command === "save_item") {
          const index = items.findIndex((item) => item.id === args.item.id);
          if (index < 0)
            items.unshift({
              ...args.item,
              id: crypto.randomUUID(),
              createdAt: now,
            });
          else items[index] = args.item;
        } else if (command === "rename_todo") {
          const value = items.find((item) => item.id === args.id);
          if (value) value.title = args.title;
        } else if (command === "toggle_item") {
          const value = items.find((item) => item.id === args.id);
          if (value) value.done = !value.done;
        } else if (command === "remove_item")
          items = items.filter((item) => item.id !== args.id);
        else if (command === "start_timer")
          items.unshift(
            item(crypto.randomUUID(), "timer", args.title, {
              state: "running",
              durationMs: args.minutes * 60000,
              remainingMs: args.minutes * 60000,
              dueAt: now + args.minutes * 60000,
            }),
          );
        else if (command === "timer_action") {
          const value = items.find((item) => item.id === args.id);
          if (value)
            value.state = args.action === "resume" ? "running" : "paused";
        } else if (
          [
            "open_path",
            "pet_show",
            "pet_hide",
            "pet_copy",
            "pet_hold",
          ].includes(command)
        )
          return;
        else throw Error("Unexpected test command: " + command);
        root.fixtureEmit("data-changed", null);
      },
    };
  };
  await page.addInitScript({
    content:
      "window.__name = (value) => value;(" +
      initialize.toString() +
      ")(" +
      JSON.stringify({ theme, empty }) +
      ");",
  });
}
