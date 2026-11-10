import { features, pageEnabled } from "./features";
import {
  useEffect,
  useRef,
  useState,
  lazy,
  Suspense,
  type FormEvent,
  type ReactNode,
} from "react";
import { call, native, previewReconcile } from "./bridge";
import {
  emptyItem,
  formatDuration,
  parseCapture,
  validateBackup,
  type Item,
  type Snapshot,
} from "./domain";
import { destinationPage, type PanelRequest } from "./panelNavigation";
import ProjectResume from "./ProjectResume";
import TaskChecklist from "./TaskChecklist";
import type { AgentTask } from "./AgentPanel";
import { pageInfo, type Page } from "./uiNavigation";
const ScreenHelper = lazy(() => import("./ScreenHelper"));
const AgentPanel = lazy(() => import("./AgentPanel"));
type FormDraft = {
  title: string;
  minutes: string;
  due: string;
  projectPath: string;
  note: string;
  edit: Item | null;
};
const blankDraft = (): FormDraft => ({
  title: "",
  minutes: "25",
  due: "",
  projectPath: "",
  note: "",
  edit: null,
});

const paths: Record<string, string> = {
  Fill: "M4 4h16v16H4zM4 9h16M8 14h8M8 17h5",
  Codex: "M8 7l-5 5 5 5M16 7l5 5-5 5M14 4l-4 16",
  Cat: "M4 10L3 3l6 4a9 9 0 0 1 6 0l6-4-1 7v5c0 5-16 5-16 0zM8 12h.01M16 12h.01M10 15l2 2 2-2M1 15h5M18 15h5",
  Activity: "M3 12h4l3-8 4 16 3-8h4",
  Tasks: "M9 5h12M9 12h12M9 19h12M3 5h.01M3 12h.01M3 19h.01",
  Projects: "M3 7V5h6l2 2h10v13H3z",
  Shelf: "M4 4h16v16H4zM4 12h16M9 4v8",
  Music:
    "M9 18V5l12-2v13M9 8l12-2M9 18a3 3 0 1 1-3-3c2 0 3 1 3 3M21 16a3 3 0 1 1-3-3c2 0 3 1 3 3",
  Settings:
    "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M5 19l2-2M17 7l2-2",
  Search: "M21 21l-5-5M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14",
  Plus: "M12 5v14M5 12h14",
  Minus: "M5 12h14",
  Arrow: "M7 17L17 7M7 7h10v10",
  Trash: "M3 6h18M9 6V3h6v3M6 6l1 15h10l1-15",
  Pause: "M8 5v14M16 5v14",
  Play: "M8 4l12 8-12 8z",
  Check: "M5 12l4 4L19 6",
  Clock: "M12 8v5l3 2M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18",
  Close: "M6 6l12 12M18 6L6 18",
  Download: "M12 3v12M7 10l5 5 5-5M4 17v4h16v-4",
  Refresh: "M20 7a8 8 0 1 0 1 8M20 2v6h-6",
};
function Icon({ name, size = 18 }: { name: string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name] || paths.Activity} />
    </svg>
  );
}
function Empty({
  icon,
  title,
  children,
}: {
  icon: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="empty">
      <Icon name={icon} size={28} />
      <h3>{title}</h3>
      <p>{children}</p>
    </div>
  );
}
function TimerRow({
  item,
  action,
}: {
  item: Item;
  action: (command: string, args: Record<string, unknown>) => Promise<void>;
}) {
  const [left, setLeft] = useState(item.remainingMs);
  const anchor = useRef({ left: item.remainingMs, time: performance.now() });
  useEffect(() => {
    anchor.current = {
      left: native ? item.remainingMs : Math.max(0, item.dueAt - Date.now()),
      time: performance.now(),
    };
    setLeft(item.state === "running" ? anchor.current.left : item.remainingMs);
    if (item.state !== "running") return;
    const tick = () =>
      setLeft(
        Math.max(
          0,
          anchor.current.left - (performance.now() - anchor.current.time),
        ),
      );
    const t = window.setInterval(tick, 1000);
    return () => clearInterval(t);
  }, [item.remainingMs, item.state, item.dueAt]);
  return (
    <div className="row timer-row">
      <div className="row-icon">
        <Icon name="Clock" />
      </div>
      <div className="grow">
        <strong>{item.title}</strong>
        <span>
          {item.state === "finished"
            ? "Completed"
            : item.state === "paused"
              ? "Paused"
              : "Focus in progress"}
        </span>
      </div>
      <output className="countdown" aria-label="Time remaining">
        {formatDuration(left)}
      </output>
      {item.state !== "finished" && (
        <button
          className="icon-button"
          title={item.state === "running" ? "Pause timer" : "Resume timer"}
          onClick={() =>
            void action("timer_action", {
              id: item.id,
              action: item.state === "running" ? "pause" : "resume",
            })
          }
        >
          <Icon name={item.state === "running" ? "Pause" : "Play"} />
        </button>
      )}
      <button
        className="icon-button"
        title="Reset timer"
        onClick={() =>
          void action("timer_action", { id: item.id, action: "reset" })
        }
      >
        <Icon name="Refresh" />
      </button>
      {item.state !== "running" && (
        <button
          className="icon-button danger"
          title="Remove timer"
          onClick={() => void action("remove_item", { id: item.id })}
        >
          <Icon name="Trash" />
        </button>
      )}
    </div>
  );
}
export default function App() {
  const [data, setData] = useState<Snapshot | null>(null),
    [page, setPageState] = useState<Page>("Activity"),
    [error, setError] = useState(""),
    [notice, setNotice] = useState(""),
    [busy, setBusy] = useState(false),
    [capture, setCapture] = useState(""),
    [tab, setTabState] = useState("Todos");
  const [title, setTitle] = useState(""),
    [minutes, setMinutes] = useState("25"),
    [due, setDue] = useState(""),
    [projectPath, setProjectPath] = useState(""),
    [note, setNote] = useState(""),
    [edit, setEdit] = useState<Item | null>(null);
  const [providers, setProviders] = useState<
      { id: string; installed: boolean; path: string; description: string }[]
    >([]),
    [sessions, setSessions] = useState<Record<string, unknown>[]>([]),
    [sessionNote, setSessionNote] = useState(""),
    [search, setSearch] = useState(""),
    [root, setRoot] = useState(""),
    [results, setResults] = useState<
      { name: string; path: string; folder: boolean }[]
    >([]),
    [searching, setSearching] = useState(false);
  const [agentTasks, setAgentTasks] = useState<AgentTask[]>([]);
  const [focusTaskId, setFocusTaskId] = useState("");
  const [selectedProjectId, setSelectedProjectId] = useState("");
  const [projectComposer, setProjectComposer] = useState(false);
  const [noteComposer, setNoteComposer] = useState(false);
  const [fillVisited, setFillVisited] = useState(false);
  const [navigationOpen, setNavigationOpen] = useState(false);
  const drafts = useRef<Record<string, FormDraft>>({});
  const formScope = useRef({
    page,
    tab,
    draft: { title, minutes, due, projectPath, note, edit },
  });
  formScope.current = {
    page,
    tab,
    draft: { title, minutes, due, projectPath, note, edit },
  };
  function setPage(nextPage: Page, taskTab?: string, focusHeader = true) {
    if (!pageEnabled(nextPage)) return;
    const current = formScope.current;
    const key = (value: Page, tab: string) =>
      value === "Tasks" ? value + ":" + tab : value;
    drafts.current[key(current.page, current.tab)] = current.draft;
    const nextTab = taskTab || current.tab;
    const nextDraft = drafts.current[key(nextPage, nextTab)] || blankDraft();
    formScope.current = { page: nextPage, tab: nextTab, draft: nextDraft };
    setPageState(nextPage);
    setTabState(nextTab);
    setTitle(nextDraft.title);
    setMinutes(nextDraft.minutes);
    setDue(nextDraft.due);
    setProjectPath(nextDraft.projectPath);
    setNote(nextDraft.note);
    setEdit(nextDraft.edit);
    setNavigationOpen(false);
    if (focusHeader)
      requestAnimationFrame(() =>
        document.querySelector<HTMLElement>("h1")?.focus(),
      );
    if (nextPage === "Fill") setFillVisited(true);
  }
  function setTab(nextTab: string) {
    setPage("Tasks", nextTab, false);
  }
  async function saveTodo(command: string, args: Record<string, unknown>) {
    const blocking = command !== "rename_todo";
    if (blocking) setBusy(true);
    try {
      await action(command, args);
      return true;
    } catch {
      return false;
    } finally {
      if (blocking) setBusy(false);
    }
  }
  const [navigation, setNavigation] = useState<PanelRequest | null>(null);
  const latestNavigation = useRef(0);
  function applyNavigation(request: PanelRequest | null) {
    if (!request || request.id <= latestNavigation.current) return;
    latestNavigation.current = request.id;
    setNavigation(request);
    setPage(
      destinationPage(request.destination),
      request.destination === "timers"
        ? "Timers"
        : request.destination === "reminders"
          ? "Reminders"
          : undefined,
      false,
    );
  }
  function acknowledgeNavigation(id: number) {
    setNavigation((current) => (current?.id === id ? null : current));
    if (native)
      void call("acknowledge_navigation", { id }).catch((error) =>
        setError(String(error)),
      );
  }
  useEffect(() => {
    if (
      !data ||
      !navigation ||
      [
        "screenCapture",
        "screenExplain",
        ...(features.codexChats ? ["ask", "codex"] : []),
      ].includes(navigation.destination)
    )
      return;
    (navigation.destination === "resume"
      ? document.getElementById("project-resume-heading")
      : document.querySelector<HTMLElement>("h1")
    )?.focus();
    acknowledgeNavigation(navigation.id);
  }, [data !== null, navigation, page]);
  const captureRef = useRef<HTMLInputElement>(null),
    importRef = useRef<HTMLInputElement>(null),
    refreshSeq = useRef(0),
    searchSeq = useRef(0);
  const refresh = async () => {
    const n = ++refreshSeq.current;
    try {
      const s = await call<Snapshot>("snapshot");
      if (n === refreshSeq.current) setData(s);
    } catch (e) {
      setError(String(e));
    }
  };
  useEffect(() => {
    let alive = true;
    let cleanup = () => {};
    void refresh();
    void call<typeof providers>("providers")
      .then((p) => {
        if (alive) setProviders(p);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      });
    const onChange = () => void refresh();
    if (native) {
      void import("@tauri-apps/api/event").then(async ({ listen }) => {
        const unlisten = await listen("data-changed", onChange);
        const offCodex = await listen("open-codex", () => {
          if (alive) setPage("Codex");
        });
        const offNavigation = await listen<PanelRequest>(
          "panel-navigation",
          (event) => {
            if (alive) applyNavigation(event.payload);
          },
        );
        if (alive) {
          cleanup = () => {
            unlisten();
            offCodex();
            offNavigation();
          };
          void call<PanelRequest | null>("panel_navigation")
            .then((request) => {
              if (alive) applyNavigation(request);
            })
            .catch((error) => setError(String(error)));
          void refresh();
        } else {
          unlisten();
          offCodex();
          offNavigation();
        }
      });
    } else {
      window.addEventListener("preview-changed", onChange);
      cleanup = () => window.removeEventListener("preview-changed", onChange);
    }
    const key = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        captureRef.current?.focus();
      }
      if (e.key === "Escape") {
        setEdit(null);
        captureRef.current?.blur();
      }
    };
    window.addEventListener("keydown", key);
    return () => {
      alive = false;
      cleanup();
      window.removeEventListener("keydown", key);
    };
  }, []);
  useEffect(() => {
    if (native) return;
    previewReconcile();
    const t = window.setInterval(previewReconcile, 1000);
    return () => clearInterval(t);
  }, []);
  useEffect(() => {
    if (!data) return;
    document.documentElement.dataset.theme = data.theme;
    document.documentElement.dataset.pet = ["calico", "dog", "panda"].includes(
      data.petAppearance || "",
    )
      ? data.petAppearance
      : "calico";
  }, [data?.theme, data?.petAppearance]);
  useEffect(() => {
    if (!native) return;
    let active = true;
    let unlisten = () => {};
    void import("@tauri-apps/api/webviewWindow").then(
      async ({ getCurrentWebviewWindow }) => {
        const off = await getCurrentWebviewWindow().onDragDropEvent((e) => {
          if (e.payload.type === "drop")
            void run("stage_paths", { paths: e.payload.paths });
        });
        if (active) unlisten = off;
        else off();
      },
    );
    return () => {
      active = false;
      unlisten();
    };
  }, []);
  useEffect(() => {
    const id = ++searchSeq.current;
    setResults([]);
    if (!root || search.trim().length < 2) {
      setSearching(false);
      return;
    }
    setSearching(true);
    const t = window.setTimeout(() => {
      void call<typeof results>("search_folder", { root, query: search.trim() })
        .then((r) => {
          if (id === searchSeq.current) setResults(r);
        })
        .catch((e) => {
          if (id === searchSeq.current) setError(String(e));
        })
        .finally(() => {
          if (id === searchSeq.current) setSearching(false);
        });
    }, 200);
    return () => clearTimeout(t);
  }, [root, search]);
  async function action(command: string, args: Record<string, unknown> = {}) {
    setError("");
    try {
      await call(command, args);
      await refresh();
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }
  const run = (command: string, args: Record<string, unknown> = {}) =>
    action(command, args).catch(() => {});
  async function submitCapture(e: FormEvent) {
    e.preventDefault();
    const parsed = parseCapture(capture);
    if (parsed.type === "error") {
      setError(parsed.message);
      return;
    }
    setBusy(true);
    try {
      if (parsed.type === "timer")
        await action("start_timer", {
          title: parsed.title,
          minutes: parsed.minutes,
          projectId: "",
        });
      else if (parsed.type === "reminder") {
        setEdit(null);
        setTitle(parsed.title);
        setDue(
          new Date(parsed.dueAt - new Date().getTimezoneOffset() * 60000)
            .toISOString()
            .slice(0, 16),
        );
        setPage("Tasks", "Reminders");
        setNotice(
          `Review reminder: ${new Date(parsed.dueAt).toLocaleString()}. Save below to confirm.`,
        );
      } else
        await action("save_item", { item: emptyItem("todo", parsed.title) });
      setCapture("");
      if (parsed.type !== "reminder")
        setNotice(
          parsed.type === "timer" ? "Timer started." : "Todo captured.",
        );
    } catch {
    } finally {
      setBusy(false);
    }
  }
  async function saveForm(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    try {
      if (page === "Projects") {
        const i = edit || emptyItem("project", title);
        await action("save_item", {
          item: { ...i, title, path: projectPath, note },
        });
        setEdit(null);
        setProjectPath("");
        setNote("");
      } else if (page === "Shelf") {
        await action("save_item", {
          item: { ...emptyItem("shelf", title), note },
        });
        setNote("");
      } else if (tab === "Timers") {
        await action("start_timer", {
          title,
          minutes: Number(minutes),
          projectId: "",
        });
      } else {
        const i =
          edit || emptyItem(tab === "Reminders" ? "reminder" : "todo", title);
        await action("save_item", {
          item: {
            ...i,
            title,
            dueAt: tab === "Reminders" ? new Date(due).getTime() : 0,
          },
        });
        setEdit(null);
        setDue("");
      }
      setTitle("");
      setNotice("Saved locally.");
    } catch {
    } finally {
      setBusy(false);
    }
  }
  async function discover() {
    setBusy(true);
    setError("");
    try {
      const r = await call<{
        data: Record<string, unknown>[];
        nextCursor?: string;
      }>("codex_sessions");
      setSessions(r.data || []);
      setSessionNote(
        `Stored history read at ${new Date().toLocaleTimeString()}. Not a live subscription.${r.nextCursor ? " Showing the first 25." : ""}`,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function backup() {
    try {
      const text = await call<string>("export_backup");
      const url = URL.createObjectURL(
        new Blob([text], { type: "application/json" }),
      );
      const a = document.createElement("a");
      a.href = url;
      a.download = `buddy-backup-${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      setNotice("Backup exported. Keep it somewhere private.");
    } catch (e) {
      setError(String(e));
    }
  }
  const items = data?.items || [],
    attention = items.filter((i) => i.kind === "attention" && !i.done),
    todos = items.filter((i) => i.kind === "todo"),
    projects = items.filter((i) => i.kind === "project"),
    timers = items.filter((i) => i.kind === "timer"),
    shelf = items.filter((i) => i.kind === "shelf");
  const clearForm = () => {
    setTitle("");
    setDue("");
    setEdit(null);
    setNote("");
    setProjectPath("");
  };
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setPage("Activity");
          }}
        >
          <span className="brand-mark">
            <Icon name="Cat" size={24} />
          </span>
          <span>
            Desktop Buddy<small>Your local desktop companion</small>
          </span>
        </a>
        <button
          className="navigation-toggle small-button"
          aria-expanded={navigationOpen}
          aria-controls="main-navigation"
          onClick={() => setNavigationOpen(!navigationOpen)}
        >
          Tools <Icon name={navigationOpen ? "Minus" : "Plus"} size={16} />
        </button>
        <nav
          id="main-navigation"
          className={navigationOpen ? "is-open" : ""}
          aria-label="Main navigation"
        >
          {(
            [
              "Activity",
              "Tasks",
              "Projects",
              "Shelf",
              "Codex",
              "Fill",
              "Music",
            ] as Page[]
          )
            .filter(pageEnabled)
            .map((p) => (
              <button
                key={p}
                aria-current={page === p ? "page" : undefined}
                className={page === p ? "nav-item selected" : "nav-item"}
                onClick={() => {
                  setPage(p);
                }}
              >
                <Icon name={p} />
                <span>{pageInfo[p].title}</span>
                {p === "Activity" && attention.length > 0 && (
                  <b aria-label={`${attention.length} alerts`}>
                    {attention.length}
                  </b>
                )}
                {p === "Codex" && (data?.agentAttention || 0) > 0 && (
                  <b aria-label={`${data?.agentAttention} need attention`}>
                    {data?.agentAttention}
                  </b>
                )}
              </button>
            ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-indicator">
            <span />
            Local first<small>No AI calls for everyday tools</small>
          </div>
          <button
            className={page === "Settings" ? "nav-item selected" : "nav-item"}
            aria-current={page === "Settings" ? "page" : undefined}
            onClick={() => setPage("Settings")}
          >
            <Icon name="Settings" />
            Settings
          </button>
          <div className="shortcut-hint">
            <kbd>Ctrl</kbd>
            <kbd>Alt</kbd>
            <kbd>P</kbd>
            <span>Open from anywhere</span>
          </div>
        </div>
      </aside>
      <main id="main-content" tabIndex={-1}>
        <header className="toolbar">
          <form onSubmit={(e) => void submitCapture(e)} className="capture">
            <Icon name="Plus" />
            <input
              ref={captureRef}
              aria-label="Quick capture"
              placeholder="Add a todo, or type timer 25m…"
              value={capture}
              maxLength={240}
              onChange={(e) => setCapture(e.target.value)}
            />
            <button
              disabled={busy || !capture.trim()}
              type="submit"
              title="Add quick capture"
              aria-label="Add quick capture"
            >
              <Icon name="Plus" />
            </button>
            <kbd>Ctrl K</kbd>
          </form>
          <span className="runtime-label">
            {native ? "DESKTOP" : "PREVIEW"}
          </span>
        </header>
        {!native && (
          <div className="preview-banner">
            Browser preview · saved here only. Desktop integrations and
            background notifications require the Windows app.
          </div>
        )}
        <div className="content">
          <div className="page-heading">
            <div>
              <h1 tabIndex={-1}>{pageInfo[page].title}</h1>
              <p>{pageInfo[page].description}</p>
            </div>
          </div>
          {error && (
            <div className="message error" role="alert">
              <span>{error}</span>
              {!data && (
                <button className="small-button" onClick={() => void refresh()}>
                  Retry loading
                </button>
              )}
              <button
                className="icon-button"
                title="Dismiss error"
                onClick={() => setError("")}
              >
                <Icon name="Close" />
              </button>
            </div>
          )}
          {data?.warning && (
            <div className="message warning">{data.warning}</div>
          )}
          {notice && (
            <div className="message notice" role="status">
              <span>{notice}</span>
              <button
                className="icon-button"
                title="Dismiss notice"
                onClick={() => setNotice("")}
              >
                <Icon name="Close" />
              </button>
            </div>
          )}
          {!data ? (
            <div className="loading" aria-label="Loading local data">
              <div />
              <div />
              <div />
            </div>
          ) : (
            <>
              <div hidden={page !== "Fill"} className="page-pane">
                {(fillVisited || page === "Fill") && (
                  <Suspense fallback={<p>Loading page helper...</p>}>
                    <ScreenHelper
                      visible={page === "Fill"}
                      navigation={navigation}
                      onNavigated={acknowledgeNavigation}
                    />
                  </Suspense>
                )}
              </div>
              {features.codexChats && (
                <div hidden={page !== "Codex"} className="page-pane codex-pane">
                  <Suspense
                    fallback={
                      <p className="explanation">Loading Codex workspace...</p>
                    }
                  >
                    <AgentPanel
                      visible={page === "Codex"}
                      navigation={navigation}
                      onNavigated={acknowledgeNavigation}
                      onTasksChange={setAgentTasks}
                      focusTaskId={focusTaskId}
                      projects={projects}
                      shelf={shelf}
                      todos={todos}
                    />
                  </Suspense>
                  <details className="connections-disclosure">
                    <summary>Connections and stored history</summary>
                    <section>
                      <div className="section-heading">
                        <h2>Agent connections</h2>
                        <span className="muted">
                          Capability checked, never assumed
                        </span>
                      </div>
                      {providers.map((p) => (
                        <div className="provider" key={p.id}>
                          <div className="provider-avatar">
                            {p.id === "codex" ? "C" : "A"}
                          </div>
                          <div className="grow">
                            <strong>
                              {p.id === "codex" ? "Codex" : "Antigravity"}
                            </strong>
                            <p>{p.description}</p>
                            <span className="status-tag">
                              {p.installed ? "CLI detected" : "Not connected"}
                            </span>
                          </div>
                          {p.id === "codex" && (
                            <button
                              className="small-button"
                              disabled={!p.installed || busy}
                              onClick={() => void discover()}
                            >
                              {busy ? "Reading…" : "Read stored sessions"}
                            </button>
                          )}
                        </div>
                      ))}
                      {sessionNote && (
                        <p className="explanation">{sessionNote}</p>
                      )}
                      {sessions.map((s, i) => (
                        <div className="row session" key={String(s.id || i)}>
                          <div className="grow">
                            <strong>
                              {String(
                                s.name ||
                                  s.preview ||
                                  "Untitled stored session",
                              ).slice(0, 150)}
                            </strong>
                            <span>
                              {typeof s.cwd === "string" ? s.cwd : ""}
                            </span>
                          </div>
                          <span className="status-tag">
                            Stored · live state unverified
                          </span>
                        </div>
                      ))}
                      <p className="explanation">
                        Reading history does not attach to desktop work. No
                        prompts are sent and no sessions are resumed.
                      </p>
                    </section>
                  </details>
                </div>
              )}
              {page === "Activity" && (
                <>
                  {features.codexChats && (data.agentAttention || 0) > 0 && (
                    <div className="attention-banner">
                      <div>
                        <strong>Codex needs your attention</strong>
                        <p>Review a decision, question or task outcome.</p>
                      </div>
                      <button
                        className="small-button"
                        onClick={() => setPage("Codex")}
                      >
                        Review Codex
                      </button>
                    </div>
                  )}
                  <section>
                    <div className="section-heading">
                      <h2>
                        Needs attention{" "}
                        <span className="number">{attention.length}</span>
                      </h2>
                      <span className="muted">Only things you can act on</span>
                    </div>
                    {attention.length ? (
                      attention.map((i) => (
                        <div className="row" key={i.id}>
                          <span className="attention-dot" />
                          <div className="grow">
                            <strong>{i.title}</strong>
                            <span>
                              {i.source} ·{" "}
                              {new Date(i.createdAt).toLocaleString()}
                            </span>
                          </div>
                          <button
                            className="small-button"
                            onClick={() =>
                              void run("toggle_item", { id: i.id })
                            }
                          >
                            Mark seen
                          </button>
                        </div>
                      ))
                    ) : (
                      <Empty icon="Check" title="No alerts to review.">
                        Timer and reminder alerts will appear here when they are
                        due.
                      </Empty>
                    )}
                  </section>
                  {timers.some((i) => i.state === "running") && (
                    <section>
                      <div className="section-heading">
                        <h2>In focus</h2>
                        <button
                          className="text-button"
                          onClick={() => {
                            setPage("Tasks", "Timers");
                          }}
                        >
                          View timers <Icon name="Arrow" size={14} />
                        </button>
                      </div>
                      {timers
                        .filter((i) => i.state === "running")
                        .map((i) => (
                          <TimerRow key={i.id} item={i} action={run} />
                        ))}
                    </section>
                  )}
                  <section className="overview-todos">
                    <div className="section-heading">
                      <h2>Next on your list</h2>
                      <button
                        className="text-button"
                        onClick={() => setPage("Tasks", "Todos")}
                      >
                        All todos <Icon name="Arrow" size={14} />
                      </button>
                    </div>
                    {todos
                      .filter((item) => !item.done)
                      .slice(0, 4)
                      .map((item) => (
                        <label className="overview-todo" key={item.id}>
                          <input
                            type="checkbox"
                            checked={false}
                            onChange={() =>
                              void run("toggle_item", { id: item.id })
                            }
                          />
                          <span>{item.title}</span>
                        </label>
                      ))}
                    {!todos.some((item) => !item.done) && (
                      <p className="list-empty">
                        Your list is clear. Add a todo in the quick-add bar
                        above.
                      </p>
                    )}
                  </section>
                  {features.workspaces && (
                    <section className="overview-projects">
                      <div className="section-heading">
                        <h2>Continue a project</h2>
                        <button
                          className="text-button"
                          onClick={() => setPage("Projects")}
                        >
                          All projects <Icon name="Arrow" size={14} />
                        </button>
                      </div>
                      {projects.slice(0, 3).map((project) => (
                        <button
                          key={project.id}
                          className="project-resume-link"
                          onClick={() => {
                            setSelectedProjectId(project.id);
                            setPage("Projects");
                          }}
                        >
                          <Icon name="Projects" />
                          <span>
                            <strong>{project.title}</strong>
                            <span>
                              {project.note || "Add a next step in Projects."}
                            </span>
                          </span>
                          <Icon name="Arrow" size={16} />
                        </button>
                      ))}
                      {!projects.length && (
                        <p className="list-empty">
                          Save a folder in Projects to keep its next step close.
                        </p>
                      )}
                    </section>
                  )}
                </>
              )}
              {page === "Tasks" && (
                <>
                  <div className="tabs" role="group" aria-label="Task type">
                    {["Todos", "Timers", "Reminders"].map((t) => (
                      <button
                        aria-pressed={tab === t}
                        key={t}
                        className={tab === t ? "active" : ""}
                        onClick={() => {
                          setTab(t);
                        }}
                      >
                        {t}
                      </button>
                    ))}
                  </div>
                  <form className="editor" onSubmit={(e) => void saveForm(e)}>
                    <label className="grow">
                      {tab === "Timers" ? "What are you focusing on?" : "Title"}
                      <input
                        required={tab !== "Timers"}
                        value={title}
                        maxLength={240}
                        onChange={(e) => setTitle(e.target.value)}
                        placeholder={
                          tab === "Todos"
                            ? "A small next step…"
                            : tab === "Timers"
                              ? "Focus session"
                              : "Remember to…"
                        }
                      />
                    </label>
                    {tab === "Timers" && (
                      <label className="duration">
                        Minutes
                        <input
                          type="number"
                          min="1"
                          max="1440"
                          required
                          value={minutes}
                          onChange={(e) => setMinutes(e.target.value)}
                        />
                      </label>
                    )}
                    {tab === "Reminders" && (
                      <label>
                        When <small>local time</small>
                        <input
                          type="datetime-local"
                          required
                          value={due}
                          onChange={(e) => setDue(e.target.value)}
                        />
                      </label>
                    )}
                    <button className="primary" disabled={busy}>
                      {tab === "Timers"
                        ? "Start timer"
                        : edit
                          ? "Save changes"
                          : "Add " + (tab === "Todos" ? "todo" : "reminder")}
                      <Icon name="Plus" size={16} />
                    </button>
                    {edit && (
                      <button
                        type="button"
                        className="small-button"
                        onClick={clearForm}
                      >
                        Cancel
                      </button>
                    )}
                  </form>
                  {tab === "Todos" ? (
                    <TaskChecklist items={todos} busy={busy} save={saveTodo} />
                  ) : tab === "Timers" ? (
                    <>
                      {timers.length ? (
                        timers.map((i) => (
                          <TimerRow key={i.id} item={i} action={run} />
                        ))
                      ) : (
                        <Empty
                          icon="Clock"
                          title="Give one thing your attention."
                        >
                          Start a timer. It keeps running when the panel is
                          closed in the native app.
                        </Empty>
                      )}
                      <p className="explanation">
                        Running timers include sleep. After quitting or
                        restarting, recovery uses the saved clock deadline;
                        manual clock changes can affect that fallback.
                      </p>
                    </>
                  ) : (
                    <>
                      {items.filter(
                        (i) =>
                          i.kind === (tab === "Todos" ? "todo" : "reminder"),
                      ).length ? (
                        items
                          .filter(
                            (i) =>
                              i.kind ===
                              (tab === "Todos" ? "todo" : "reminder"),
                          )
                          .map((i) => (
                            <div
                              className={"row " + (i.done ? "completed" : "")}
                              key={i.id}
                            >
                              {i.kind === "todo" ? (
                                <input
                                  className="checkbox"
                                  type="checkbox"
                                  checked={i.done}
                                  aria-label={`Complete ${i.title}`}
                                  onChange={() =>
                                    void run("toggle_item", { id: i.id })
                                  }
                                />
                              ) : (
                                <div className="row-icon">
                                  <Icon name="Clock" />
                                </div>
                              )}
                              <div className="grow">
                                <strong>{i.title}</strong>
                                {i.dueAt > 0 && (
                                  <span>
                                    {i.done ? "Delivered to Needs me · " : ""}
                                    {new Date(i.dueAt).toLocaleString()}
                                  </span>
                                )}
                              </div>
                              {!i.done && (
                                <button
                                  className="text-button"
                                  onClick={() => {
                                    setEdit(i);
                                    setTitle(i.title);
                                    setDue(
                                      i.dueAt
                                        ? new Date(
                                            i.dueAt -
                                              new Date().getTimezoneOffset() *
                                                60000,
                                          )
                                            .toISOString()
                                            .slice(0, 16)
                                        : "",
                                    );
                                  }}
                                >
                                  Edit
                                </button>
                              )}
                              <button
                                className="icon-button danger"
                                title={`Remove ${i.title}`}
                                onClick={() =>
                                  void run("remove_item", { id: i.id })
                                }
                              >
                                <Icon name="Trash" />
                              </button>
                            </div>
                          ))
                      ) : (
                        <Empty
                          icon={tab === "Todos" ? "Tasks" : "Clock"}
                          title={
                            tab === "Todos"
                              ? "A clear list starts here."
                              : "Remember it, then let it go."
                          }
                        >
                          {tab === "Todos"
                            ? "Add a todo, or use quick capture from any screen."
                            : "Add a one-off reminder. Due items stay in Needs me until you mark them seen."}
                        </Empty>
                      )}
                    </>
                  )}
                  <details className="help-disclosure">
                    <summary>
                      Timers and reminders while Buddy is closed
                    </summary>{" "}
                    <p className="explanation">
                      Notifications require the tray app to be running. Quit
                      stops reminders; overdue items recover next launch.
                      Windows may suppress banners.
                    </p>
                  </details>
                </>
              )}
              <div
                hidden={page !== "Projects"}
                className="page-pane projects-pane"
              >
                <details
                  className="compose-panel"
                  open={projectComposer}
                  onToggle={(event) =>
                    setProjectComposer(event.currentTarget.open)
                  }
                >
                  <summary>
                    {edit?.kind === "project"
                      ? "Edit project"
                      : "Add a project"}
                  </summary>
                  <form
                    className="editor stacked"
                    onSubmit={(e) => void saveForm(e)}
                  >
                    <div className="form-pair">
                      <label className="grow">
                        Project name
                        <input
                          required
                          value={title}
                          maxLength={240}
                          onChange={(e) => setTitle(e.target.value)}
                          placeholder="My next project"
                        />
                      </label>
                      <label className="grow">
                        Folder path
                        <input
                          required
                          value={projectPath}
                          onChange={(e) => setProjectPath(e.target.value)}
                          placeholder="C:\Projects\my-project"
                        />
                      </label>
                    </div>
                    <label>
                      Where should you pick up?
                      <textarea
                        value={note}
                        maxLength={65536}
                        onChange={(e) => setNote(e.target.value)}
                        placeholder="The next small step, a useful command, or a link…"
                        rows={2}
                      />
                    </label>
                    <div>
                      <button className="primary" disabled={busy}>
                        {edit ? "Save project" : "Save project"}
                        <Icon name="Plus" size={16} />
                      </button>
                      {edit && (
                        <button
                          type="button"
                          className="text-button"
                          onClick={clearForm}
                        >
                          Cancel
                        </button>
                      )}
                    </div>
                  </form>
                </details>
                <div className="project-workspace">
                  <div className="project-list">
                    {projects.length ? (
                      projects.map((i) => (
                        <article className="project" key={i.id}>
                          <div className="project-top">
                            <Icon name="Projects" />
                            <div className="grow">
                              <h2>{i.title}</h2>
                              <span className="path">{i.path}</span>
                            </div>
                            <button
                              className="small-button"
                              onClick={() =>
                                void run("open_path", { path: i.path })
                              }
                            >
                              Open folder <Icon name="Arrow" size={14} />
                            </button>
                          </div>
                          {i.note && <p className="resume-note">{i.note}</p>}
                          <div className="project-actions">
                            <button
                              className="text-button"
                              onClick={() => {
                                setSelectedProjectId(i.id);
                                document
                                  .getElementById("project-resume-heading")
                                  ?.focus();
                              }}
                            >
                              Resume
                            </button>
                            <button
                              className="text-button"
                              onClick={() => {
                                setProjectComposer(true);
                                setEdit(i);
                                setTitle(i.title);
                                setProjectPath(i.path);
                                setNote(i.note);
                              }}
                            >
                              Edit project
                            </button>
                            <button
                              className="text-button"
                              onClick={() => {
                                setRoot(i.path);
                                setSearch("");
                                setNotice(
                                  "Search this project using the form below.",
                                );
                              }}
                            >
                              Find files
                            </button>
                            <button
                              className="icon-button danger"
                              title={`Remove project ${i.title}`}
                              onClick={() =>
                                void run("remove_item", { id: i.id })
                              }
                            >
                              <Icon name="Trash" />
                            </button>
                          </div>
                        </article>
                      ))
                    ) : (
                      <Empty
                        icon="Projects"
                        title="Your next step deserves a home."
                      >
                        Save a folder and a short resume note. The next time you
                        switch projects, start here.
                      </Empty>
                    )}
                  </div>
                  <div className="project-detail">
                    <ProjectResume
                      projects={projects}
                      shelf={shelf}
                      todos={todos}
                      tasks={agentTasks}
                      selectedProjectId={selectedProjectId}
                      onProjectChange={setSelectedProjectId}
                      onViewTask={(id) => {
                        setFocusTaskId(id);
                        setPage("Codex", undefined, false);
                      }}
                    />
                  </div>
                </div>
                <section>
                  <div className="section-heading">
                    <h2>Find in a project</h2>
                    <span className="muted">Immediate folder only</span>
                  </div>
                  <div className="form-pair">
                    <label className="grow">
                      Project
                      <select
                        value={root}
                        onChange={(e) => setRoot(e.target.value)}
                      >
                        <option value="">Select a saved project</option>
                        {projects.map((p) => (
                          <option key={p.id} value={p.path}>
                            {p.title}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="grow">
                      Filename or folder
                      <input
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                        placeholder="At least 2 characters"
                      />
                    </label>
                  </div>
                  {searching ? (
                    <p role="status">Searching…</p>
                  ) : (
                    results.map((r, i) => (
                      <button
                        key={i}
                        className="search-result"
                        onClick={() => void run("open_path", { path: r.path })}
                      >
                        <Icon name={r.folder ? "Projects" : "Shelf"} />
                        <span>{r.name}</span>
                        <Icon name="Arrow" size={14} />
                      </button>
                    ))
                  )}
                  {root &&
                    search.length >= 2 &&
                    !searching &&
                    !results.length && (
                      <p className="explanation">
                        No matches in this directory. Whole-disk Everything
                        search is not connected yet.
                      </p>
                    )}
                </section>
              </div>
              {page === "Shelf" && (
                <>
                  <div className="drop-zone">
                    <Icon name="Shelf" size={28} />
                    <strong>
                      {native
                        ? "Drop files anywhere in this window"
                        : "File references need the desktop app"}
                    </strong>
                    <p>
                      No moving, copying, or uploading your originals. Up to 50
                      references.
                    </p>
                  </div>
                  <details
                    className="compose-panel"
                    open={noteComposer}
                    onToggle={(event) =>
                      setNoteComposer(event.currentTarget.open)
                    }
                  >
                    <summary>Add a note</summary>
                    <form
                      className="editor stacked"
                      onSubmit={(e) => void saveForm(e)}
                    >
                      <label>
                        Note title
                        <input
                          required
                          value={title}
                          maxLength={240}
                          onChange={(e) => setTitle(e.target.value)}
                          placeholder="Something worth keeping handy"
                        />
                      </label>
                      <label>
                        Text
                        <textarea
                          required
                          value={note}
                          maxLength={65536}
                          onChange={(e) => setNote(e.target.value)}
                          rows={3}
                        />
                      </label>
                      <div>
                        <button
                          className="primary"
                          disabled={busy || shelf.length >= 50}
                        >
                          Add to shelf
                          <Icon name="Plus" size={16} />
                        </button>
                      </div>
                    </form>
                  </details>
                  {shelf.map((i) => (
                    <article className="shelf-item" key={i.id}>
                      <div className="row">
                        <div className="grow">
                          <strong>{i.title}</strong>
                          {i.path && <span className="path">{i.path}</span>}
                        </div>
                        {i.path && (
                          <button
                            className="small-button"
                            disabled={!native}
                            onClick={() =>
                              void run("pet_hold", { paths: [i.path] })
                            }
                          >
                            Hold with pet
                          </button>
                        )}
                        {i.path && (
                          <button
                            className="small-button"
                            onClick={() =>
                              void run("open_path", { path: i.path })
                            }
                          >
                            Open <Icon name="Arrow" size={14} />
                          </button>
                        )}
                        <button
                          className="icon-button danger"
                          title={`Remove shelf item ${i.title}`}
                          onClick={() => void run("remove_item", { id: i.id })}
                        >
                          <Icon name="Trash" />
                        </button>
                      </div>
                      {i.note && <p>{i.note}</p>}
                      <label>
                        Project for this reference
                        <select
                          value={
                            projects.some(
                              (project) => project.id === i.projectId,
                            )
                              ? i.projectId
                              : ""
                          }
                          disabled={busy}
                          onChange={(event) =>
                            void run("save_item", {
                              item: { ...i, projectId: event.target.value },
                            })
                          }
                        >
                          <option value="">Unassigned</option>
                          {projects.map((project) => (
                            <option key={project.id} value={project.id}>
                              {project.title}
                            </option>
                          ))}
                        </select>
                      </label>
                    </article>
                  ))}
                </>
              )}
              {page === "Music" && (
                <>
                  <div className="music-intro">
                    <Icon name="Music" size={36} />
                    <h2>Automatic sync is not connected</h2>
                    <p>
                      Playback detection and automatic synchronized lyrics are a
                      later milestone. This module makes no network requests and
                      starts no background player.
                    </p>
                    <span className="status-tag">Not connected</span>
                  </div>
                  <section>
                    <h2>Open your own LRC file</h2>
                    <p className="explanation">
                      Preview timestamps and lyrics locally. This manual reader
                      is a fallback; it does not track Spotify playback.
                    </p>
                    <LrcReader />
                  </section>
                </>
              )}
              {page === "Settings" && (
                <>
                  <section className="settings-section">
                    <h2>Appearance</h2>
                    <label className="setting-row">
                      <span>
                        Quiet mode{" "}
                        <small>
                          Static pet and silent alerts; timers keep running.
                        </small>
                      </span>
                      <input
                        type="checkbox"
                        checked={!!data.quietMode}
                        onChange={(e) =>
                          void run("set_quiet_mode", {
                            enabled: e.target.checked,
                          })
                        }
                      />
                    </label>
                    <label className="setting-row">
                      Theme
                      <select
                        value={data.theme}
                        aria-label="Theme"
                        onChange={(e) =>
                          void run("set_theme", { theme: e.target.value })
                        }
                      >
                        <option value="dark">Warm charcoal</option>
                        <option value="light">Light</option>
                        <option value="system">Follow system</option>
                      </select>
                    </label>
                  </section>
                  <section className="settings-section">
                    <h2>Data and backups</h2>
                    <p className="explanation">
                      Todos, projects, notes, timers, and reminder history stay
                      on this device. Backups contain private text and file
                      paths.
                    </p>
                    <div className="button-row">
                      <button
                        className="small-button"
                        onClick={() => void backup()}
                      >
                        <Icon name="Download" />
                        Export backup
                      </button>
                      <button
                        className="small-button"
                        onClick={() => importRef.current?.click()}
                      >
                        Import backup
                      </button>
                    </div>
                    <input
                      ref={importRef}
                      hidden
                      type="file"
                      accept=".json,application/json"
                      onChange={(e) => {
                        const file = e.target.files?.[0];
                        e.target.value = "";
                        if (!file) return;
                        void (async () => {
                          try {
                            if (file.size > 5 * 1024 * 1024)
                              throw new Error("Backup exceeds 5 MiB.");
                            const text = await file.text();
                            const items = validateBackup(text);
                            const agentCount = (
                              JSON.parse(text).agentTasks || []
                            ).length;
                            if (
                              !window.confirm(
                                `Merge ${items.length} local items and ${agentCount} agent receipts? Existing IDs are kept; running timers import paused. Active agent records restore as unknown and are never replayed.`,
                              )
                            )
                              return;
                            const n = await call<number>("import_backup", {
                              data: text,
                            });
                            await refresh();
                            setNotice(`Imported ${n} new items.`);
                          } catch (e) {
                            setError(String(e));
                          }
                        })();
                      }}
                    />
                    <p className="data-path">{data.dataPath}</p>
                  </section>
                  <section className="settings-section">
                    <h2>Desktop pet</h2>
                    <p className="explanation">
                      Click the floating pet for the compact command bar; /tools
                      opens this window. Drop up to 20 local files or folders
                      onto it to hold references until cleared or handed off.
                      Drag a loaded pet to copy files into a folder or
                      compatible app; Alt-drag moves the pet. Expiry never
                      deletes originals.
                    </p>
                    <div className="button-row">
                      <button
                        className="small-button"
                        disabled={!native}
                        onClick={() => void action("pet_show")}
                      >
                        Show pet
                      </button>
                      <button
                        className="small-button"
                        disabled={!native}
                        onClick={() => void action("pet_hide")}
                      >
                        Hide pet
                      </button>
                      <button
                        className="small-button"
                        disabled={!native}
                        onClick={() => void action("pet_copy")}
                      >
                        Copy held files
                      </button>
                    </div>
                    <details className="help-disclosure">
                      <summary>Runtime and connection details</summary>{" "}
                      <p className="explanation">
                        No continuous animation loop, local AI model, clipboard
                        watcher, or whole-disk crawler. The native timer and
                        reminder scheduler wakes for deadlines. Closing the
                        window releases its panel; the tray stays running.
                      </p>
                      <p className="explanation">
                        Build 0.4.0 · Native resource targets are not a
                        benchmark result. Agent history is read only on request.
                        Automatic monitoring, Everything and Spotify sync are
                        not connected yet.
                      </p>
                    </details>
                  </section>
                </>
              )}
            </>
          )}
        </div>
        <footer>
          <span>Desktop Buddy</span>
          <span>
            {native ? "Local desktop utility" : "Browser preview"} · v0.4.0
          </span>
        </footer>
      </main>
    </div>
  );
}
function LrcReader() {
  const [lines, setLines] = useState<{ time: string; text: string }[]>([]),
    [error, setError] = useState("");
  return (
    <>
      <label className="file-label">
        Choose .lrc file
        <input
          type="file"
          accept=".lrc,text/plain"
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (!f) return;
            if (f.size > 1024 * 1024) {
              setError("Choose a file smaller than 1 MiB.");
              return;
            }
            void f.text().then((text) => {
              setError("");
              const parsed = text
                .split(/\r?\n/)
                .flatMap((line) => {
                  const m = /^\[(\d{1,3}:\d{2}(?:\.\d{1,3})?)\](.*)$/.exec(
                    line,
                  );
                  return m ? [{ time: m[1], text: m[2] }] : [];
                })
                .slice(0, 2000);
              setLines(parsed);
              if (!parsed.length) setError("No timestamped lyric lines found.");
            });
          }}
        />
      </label>
      {error && <p role="alert">{error}</p>}
      {lines.length > 0 && (
        <ol className="lyrics">
          {lines.map((l, i) => (
            <li key={i}>
              <time>{l.time}</time>
              <span>{l.text}</span>
            </li>
          ))}
        </ol>
      )}
    </>
  );
}
