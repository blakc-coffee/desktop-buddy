import { useState, type FormEvent } from "react";
import { call } from "./bridge";
import type { Item } from "./domain";
import type { AgentTask } from "./AgentPanel";

export default function ProjectResume({
  projects,
  shelf,
  todos,
  tasks,
  selectedProjectId,
  onProjectChange,
  onViewTask,
}: {
  projects: Item[];
  shelf: Item[];
  todos: Item[];
  tasks: AgentTask[];
  selectedProjectId?: string;
  onProjectChange?: (id: string) => void;
  onViewTask?: (id: string) => void;
}) {
  const [selected, setSelected] = useState("");
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const recent = [...tasks].sort((a, b) => b.updatedAt - a.updatedAt);
  const lastProject = recent.find((task) =>
    projects.some((project) => project.id === task.projectId),
  );
  const project =
    projects.find((item) => item.id === (selectedProjectId || selected)) ??
    projects.find((item) => item.id === lastProject?.projectId) ??
    projects[0];
  return (
    <section aria-label="Resume a project">
      <div className="section-heading">
        <h2 id="project-resume-heading" tabIndex={-1}>
          Pick up a project
        </h2>
      </div>
      {project ? (
        <>
          <label>
            Project to resume
            <select
              value={project.id}
              onChange={(e) => {
                setSelected(e.target.value);
                onProjectChange?.(e.target.value);
              }}
            >
              {projects.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.title}
                </option>
              ))}
            </select>
          </label>
          <ResumeDetails
            key={project.id}
            project={project}
            shelf={shelf}
            todos={todos}
            latest={recent.find((task) => task.projectId === project.id)}
            onViewTask={onViewTask}
            draft={drafts[project.id] ?? null}
            onDraft={(value) =>
              setDrafts((old) => {
                const next = { ...old };
                if (value === null) delete next[project.id];
                else next[project.id] = value;
                return next;
              })
            }
          />
        </>
      ) : (
        <p className="explanation">
          Save a folder and a next step in Projects to start here.
        </p>
      )}
    </section>
  );
}

function ResumeDetails({
  project,
  shelf,
  todos,
  latest,
  onViewTask,
  draft,
  onDraft,
}: {
  project: Item;
  shelf: Item[];
  todos: Item[];
  latest?: AgentTask;
  onViewTask?: (id: string) => void;
  draft: string | null;
  onDraft: (value: string | null) => void;
}) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const pins = shelf.filter((item) => item.projectId === project.id);
  const openTodos = todos.filter(
    (item) => item.projectId === project.id && !item.done,
  );
  async function operation(command: string, args: Record<string, unknown>) {
    setBusy(true);
    setMessage("");
    try {
      await call(command, args);
      return true;
    } catch (error) {
      setMessage(String(error));
      return false;
    } finally {
      setBusy(false);
    }
  }
  async function saveStep(event: FormEvent) {
    event.preventDefault();
    if (
      await operation("save_item", {
        item: { ...project, note: draft ?? project.note },
      })
    ) {
      onDraft(null);
    }
  }
  return (
    <div className="resume-details">
      <span className="path">{project.path}</span>
      <button
        className="small-button"
        disabled={busy}
        onClick={() => void operation("open_path", { path: project.path })}
      >
        Open project folder
      </button>
      {message && (
        <p className="message error" role="alert">
          {message}
        </p>
      )}
      <form
        className="editor stacked"
        onSubmit={(event) => void saveStep(event)}
      >
        <label>
          Your next step
          <textarea
            rows={2}
            maxLength={65536}
            value={draft ?? project.note}
            onChange={(event) => onDraft(event.target.value)}
            placeholder="Where should you pick up next time?"
          />
        </label>
        <div>
          <button className="small-button" disabled={busy || draft === null}>
            Save next step
          </button>
        </div>
      </form>
      <h3>Pinned references</h3>
      {pins.length ? (
        pins.map((item) => (
          <div className="resume-reference" key={item.id}>
            <div className="grow">
              <strong>{item.title}</strong>
              {item.path && <span className="path">{item.path}</span>}
              {item.note && <p className="resume-note">{item.note}</p>}
            </div>
            {item.path && (
              <button
                className="small-button"
                disabled={busy}
                onClick={() => void operation("open_path", { path: item.path })}
              >
                Open
              </button>
            )}
            <button
              className="text-button"
              disabled={busy}
              onClick={() =>
                void operation("save_item", {
                  item: { ...item, projectId: "" },
                })
              }
            >
              Unpin
            </button>
          </div>
        ))
      ) : (
        <p className="explanation">
          Pin a Shelf reference to keep it with this project.
        </p>
      )}
      <label>
        Pin from Shelf
        <select
          value=""
          disabled={busy || !shelf.some((item) => !item.projectId)}
          onChange={(event) => {
            const item = shelf.find(
              (entry) => entry.id === event.target.value && !entry.projectId,
            );
            if (item)
              void operation("save_item", {
                item: { ...item, projectId: project.id },
              });
          }}
        >
          <option value="">Choose a saved reference</option>
          {shelf
            .filter((item) => !item.projectId)
            .map((item) => (
              <option key={item.id} value={item.id}>
                {item.title}
              </option>
            ))}
        </select>
      </label>
      {openTodos.length > 0 && (
        <>
          <h3>Open project to-dos</h3>
          <ul>
            {openTodos.map((item) => (
              <li key={item.id}>
                {item.title}{" "}
                <button
                  className="text-button"
                  disabled={busy}
                  aria-label={"Unlink " + item.title}
                  onClick={() =>
                    void operation("save_item", {
                      item: { ...item, projectId: "" },
                    })
                  }
                >
                  Unlink
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      <label>
        Link an open to-do
        <select
          value=""
          disabled={
            busy || !todos.some((item) => !item.projectId && !item.done)
          }
          onChange={(event) => {
            const item = todos.find(
              (entry) =>
                entry.id === event.target.value &&
                !entry.projectId &&
                !entry.done,
            );
            if (item)
              void operation("save_item", {
                item: { ...item, projectId: project.id },
              });
          }}
        >
          <option value="">Choose a to-do for this project</option>
          {todos
            .filter((item) => !item.projectId && !item.done)
            .map((item) => (
              <option key={item.id} value={item.id}>
                {item.title}
              </option>
            ))}
        </select>
      </label>
      <h3>Latest Codex task</h3>
      {latest ? (
        <>
          <p>
            <strong>{latest.title}</strong> ·{" "}
            {latest.state.replaceAll("_", " ")}
          </p>
          <p className="muted">
            Updated {new Date(latest.updatedAt).toLocaleString()}
          </p>
          {latest.output && (
            <pre className="context-preview">
              {latest.output.slice(0, 1200)}
            </pre>
          )}
          <button
            className="text-button"
            onClick={() => onViewTask?.(latest.id)}
          >
            View task details
          </button>
        </>
      ) : (
        <p className="explanation">
          Codex tasks started in this project will appear here.
        </p>
      )}
    </div>
  );
}
