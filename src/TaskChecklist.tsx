import type { Item } from "./domain";
import { TodoRow } from "./Todo";
export default function TaskChecklist({
  items,
  busy,
  save,
}: {
  items: Item[];
  busy: boolean;
  save: (command: string, args: Record<string, unknown>) => Promise<boolean>;
}) {
  const open = items.filter((item) => !item.done),
    done = items.filter((item) => item.done);
  const row = (item: Item) => (
    <TodoRow key={item.id} item={item} busy={busy} save={save} allowRemove />
  );
  return (
    <div className="task-checklist">
      {open.length ? (
        open.map(row)
      ) : (
        <p className="list-empty">
          {done.length
            ? "All caught up. Add your next task when you're ready."
            : "Add your first todo above. Click a title to edit it."}
        </p>
      )}
      {done.length > 0 && (
        <details className="completed-todos">
          <summary>
            Completed <span>{done.length}</span>
          </summary>
          {done.map(row)}
        </details>
      )}
    </div>
  );
}
