import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type CSSProperties,
} from "react";
import { call, native } from "./bridge";
import { emptyItem, type Item, type Snapshot } from "./domain";

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
  async function save(command: string, args: Record<string, unknown>) {
    const blocking = command !== "rename_todo";
    if (blocking) setBusy(true);
    setError("");
    try {
      await call(command, args);
      await refresh();
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
    <main className="todo-window" style={style}>
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
      {!loaded && !error && <p role="status">Loading your checklist…</p>}
      {loaded && !items.length && (
        <p className="todo-empty">Your checklist starts here.</p>
      )}
      <div className="todo-list" aria-label="Todo checklist">
        {[...items]
          .sort((a, b) => Number(a.done) - Number(b.done))
          .map((item) => (
            <TodoRow key={item.id} item={item} busy={busy} save={save} />
          ))}
      </div>
    </main>
  );
}
export function TodoRow({
  item,
  busy,
  save,
  allowRemove = false,
}: {
  item: Item;
  busy: boolean;
  save: (command: string, args: Record<string, unknown>) => Promise<boolean>;
  allowRemove?: boolean;
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
    <div className={"todo-check-row" + (item.done ? " is-done" : "")}>
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
      {allowRemove && (
        <button
          className="text-button todo-remove"
          disabled={busy}
          aria-label={`Remove ${item.title}`}
          onClick={() => void save("remove_item", { id: item.id })}
        >
          Remove
        </button>
      )}
    </div>
  );
}
