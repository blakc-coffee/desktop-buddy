import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type CSSProperties,
} from "react";
import { call, native } from "./bridge";
import { emptyItem, type Item, type Snapshot } from "./domain";

/** Time left on a timer: live for running ones, frozen for paused ones. */
function timeLeft(item: Item, now: number) {
  const ms = item.state === "running" ? Math.max(0, item.dueAt - now) : item.remainingMs;
  const total = Math.ceil(ms / 1000);
  const h = Math.floor(total / 3600),
    m = Math.floor(total / 60) % 60,
    s = total % 60;
  const mm = String(m).padStart(2, "0"),
    ss = String(s).padStart(2, "0");
  return h ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}
function clockLabel(item: Item, now: number) {
  return item.kind === "timer"
    ? timeLeft(item, now)
    : new Date(item.dueAt)
        .toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })
        .toLowerCase();
}
const isClock = (item: Item) =>
  (item.kind === "timer" && (item.state === "running" || item.state === "paused")) ||
  (item.kind === "reminder" && !item.done);

export default function Todo() {
  const getPalette = () =>
    (window as unknown as { __BUDDY_PALETTE__?: Record<string, string> })
      .__BUDDY_PALETTE__;
  const [palette, setPalette] = useState(getPalette);
  useEffect(() => {
    const update = () => setPalette(getPalette());
    window.addEventListener("buddy-palette", update);
    return () => window.removeEventListener("buddy-palette", update);
  }, []);
  const [items, setItems] = useState<Item[]>([]);
  const [clocks, setClocks] = useState<Item[]>([]);
  const [focusId, setFocusId] = useState("");
  const [now, setNow] = useState(Date.now());
  const ticking = clocks.some((item) => item.kind === "timer" && item.state === "running");
  useEffect(() => {
    if (!ticking) return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [ticking]);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const seq = useRef(0);
  const input = useRef<HTMLInputElement>(null);
  async function refresh() {
    const id = ++seq.current;
    try {
      const data = await call<Snapshot>("snapshot");
      if (id === seq.current) {
        setItems(data.items.filter((item) => item.kind === "todo"));
        setClocks(data.items.filter(isClock));
        setFocusId(data.focusId || "");
        setNow(Date.now());
        setLoaded(true);
      }
    } catch (e) {
      setError(String(e));
    }
  }
  useEffect(() => {
    let alive = true;
    let off = () => {};
    void refresh();
    if (native)
      void import("@tauri-apps/api/event").then(async ({ listen }) => {
        const unlisten = await listen("data-changed", () => {
          if (alive) void refresh();
        });
        if (alive) {
          off = unlisten;
          void refresh();
        } else unlisten();
      });
    else {
      const onChange = () => void refresh();
      window.addEventListener("preview-changed", onChange);
      off = () => window.removeEventListener("preview-changed", onChange);
    }
    return () => {
      alive = false;
      ++seq.current;
      off();
    };
  }, []);
  const [undoable, setUndoable] = useState(false);
  const undoTimer = useRef(0);
  useEffect(() => () => window.clearTimeout(undoTimer.current), []);
  async function save(command: string, args: Record<string, unknown>) {
    const blocking = command !== "rename_todo";
    if (blocking) setBusy(true);
    setError("");
    try {
      await call(command, args);
      await refresh();
      window.clearTimeout(undoTimer.current);
      setUndoable(command === "remove_item");
      if (command === "remove_item")
        undoTimer.current = window.setTimeout(() => setUndoable(false), 10_000);
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    } finally {
      if (blocking) setBusy(false);
    }
  }
  async function add(event: FormEvent) {
    event.preventDefault();
    if (!draft.trim() || busy) return;
    if (
      await save("save_item", {
        item: { ...emptyItem("todo", ""), title: draft.trim() },
      })
    ) {
      setDraft("");
      input.current?.focus();
    }
  }
  const style = palette
    ? (Object.fromEntries(
        Object.entries(palette).map(([key, value]) => [`--todo-${key}`, value]),
      ) as CSSProperties)
    : undefined;
  return (
    <main
      className="todo-window"
      style={style}
      onKeyDown={(event) => {
        const typing = (event.target as HTMLElement).tagName === "INPUT";
        if (undoable && !typing && event.ctrlKey && event.key === "z") {
          event.preventDefault();
          void save("undo_remove", {});
        }
      }}
    >
      <h1>Todo</h1>
      <form onSubmit={(event) => void add(event)} className="todo-capture">
        <input
          ref={input}
          aria-label="New todo"
          placeholder="Add a todo…"
          value={draft}
          maxLength={240}
          onChange={(event) => setDraft(event.target.value)}
        />
        <button disabled={busy || !draft.trim()} aria-label="Add todo">
          Add
        </button>
      </form>
      {error && (
        <p role="alert" className="todo-error">
          {error}
        </p>
      )}
      {undoable && (
        <p role="status" className="todo-undo">
          Removed.
          <button type="button" disabled={busy} onClick={() => void save("undo_remove", {})}>
            Undo
          </button>
        </p>
      )}
      {!loaded && !error && <p role="status">Loading your checklist…</p>}
      {loaded && !items.length && !clocks.length && (
        <p className="todo-empty">Your checklist starts here.</p>
      )}
      <div className="todo-list" aria-label="Todo checklist">
        {clocks
          .filter((clock) => !items.some((todo) => todo.id === clock.parentId && !todo.done))
          .map((clock) => (
            <div className="todo-clock-row" key={clock.id}>
              <span aria-hidden="true">{clock.kind === "timer" ? "⏳" : "🔔"}</span>
              <span className="todo-clock-title">{clock.title}</span>
              <ClockTag clock={clock} now={now} busy={busy} save={save} />
            </div>
          ))}
        {[...items]
          .sort(
            (a, b) =>
              Number(b.id === focusId) - Number(a.id === focusId) ||
              Number(a.done) - Number(b.done),
          )
          .map((item) => (
            <TodoRow
              key={item.id}
              item={item}
              busy={busy}
              save={save}
              allowRemove
              focus={item.id === focusId}
              tags={clocks
                .filter((clock) => clock.parentId === item.id)
                .map((clock) => (
                  <ClockTag key={clock.id} clock={clock} now={now} busy={busy} save={save} />
                ))}
            />
          ))}
      </div>
    </main>
  );
}
/** A timer's time left (click pauses or resumes) or a reminder's clock time. */
function ClockTag({
  clock,
  now,
  busy,
  save,
}: {
  clock: Item;
  now: number;
  busy: boolean;
  save: (command: string, args: Record<string, unknown>) => Promise<boolean>;
}) {
  const label = clockLabel(clock, now);
  if (clock.kind !== "timer") return <span className="todo-tag">🔔 {label}</span>;
  const running = clock.state === "running";
  return (
    <button
      type="button"
      className={"todo-tag" + (running ? " is-running" : "")}
      disabled={busy}
      aria-label={`${running ? "Pause" : "Resume"} timer ${clock.title}, ${label} left`}
      onClick={() =>
        void save("timer_action", { id: clock.id, action: running ? "pause" : "resume" })
      }
    >
      ⏳ {label}
    </button>
  );
}
export function TodoRow({
  item,
  busy,
  save,
  allowRemove = false,
  focus = false,
  tags,
}: {
  item: Item;
  busy: boolean;
  save: (command: string, args: Record<string, unknown>) => Promise<boolean>;
  allowRemove?: boolean;
  focus?: boolean;
  tags?: React.ReactNode[];
}) {
  const [title, setTitle] = useState(item.title);
  const cancelled = useRef(false);
  useEffect(() => setTitle(item.title), [item.title]);
  async function commit() {
    if (cancelled.current) {
      cancelled.current = false;
      return;
    }
    if (!title.trim()) {
      setTitle(item.title);
      return;
    }
    if (
      title.trim() !== item.title &&
      !(await save("rename_todo", { id: item.id, title: title.trim() }))
    )
      setTitle(item.title);
  }
  return (
    <div
      className={
        "todo-check-row" + (item.done ? " is-done" : "") + (focus ? " todo-focus" : "")
      }
    >
      {focus && <span className="todo-focus-label">Today's focus</span>}
      <label className="todo-check-target">
        <input
          type="checkbox"
          checked={item.done}
          disabled={busy}
          aria-label={`Complete ${item.title}`}
          onChange={() => void save("toggle_item", { id: item.id })}
        />
      </label>
      <input
        aria-label={`Edit ${item.title}`}
        title={item.title}
        value={title}
        maxLength={240}
        disabled={busy}
        onChange={(event) => setTitle(event.target.value)}
        onBlur={() => void commit()}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
          if (event.key === "Escape") {
            cancelled.current = true;
            setTitle(item.title);
            event.currentTarget.blur();
          }
        }}
      />
      {tags}
      {allowRemove && !item.done && (
        <button
          className="todo-start"
          type="button"
          title="Start 25 min focus"
          disabled={busy}
          aria-label={`Start 25 min focus for ${item.title}`}
          onClick={() => void save("start_focus", { id: item.id })}
        >
          <svg aria-hidden="true" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
            <path d="M7 3h10M7 21h10M8 3c0 5 8 5 8 9s-8 4-8 9M16 3c0 5-8 5-8 9" />
          </svg>
        </button>
      )}
      {allowRemove && (
        <button
          className="todo-remove"
          type="button"
          title="Delete todo"
          disabled={busy}
          aria-label={`Remove ${item.title}`}
          onClick={() => void save("remove_item", { id: item.id })}
        >
          <svg aria-hidden="true" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
            <path d="M3 6h18M9 6V4h6v2M5 6l1 14h12l1-14M10 10v6m4-6v6" />
          </svg>
        </button>
      )}
    </div>
  );
}
