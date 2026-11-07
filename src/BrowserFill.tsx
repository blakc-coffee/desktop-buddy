import { useEffect, useState } from "react";
import { call, native } from "./bridge";
type Connection = { id: string; title: string; origin: string };
type Field = {
  id: string;
  label: string;
  kind: string;
  value: string;
  options: { value: string; label: string }[];
};
type Capture = {
  id: string;
  page: {
    title: string;
    origin: string;
    question: string;
    fields: Field[];
    unsupportedEditors: number;
  };
};
type Plan = {
  id: string;
  explanation: string;
  assignments: { id: string; value: string }[];
};
type Model = {
  model: string;
  displayName: string;
  isDefault: boolean;
  hidden?: boolean;
};
export default function BrowserFill() {
  const [connections, setConnections] = useState<Connection[]>([]);
  const [connection, setConnection] = useState("");
  const [capture, setCapture] = useState<Capture | null>(null);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [models, setModels] = useState<Model[]>([]);
  const [model, setModel] = useState("");
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  async function operation<T>(
    label: string,
    action: () => Promise<T>,
  ): Promise<T | undefined> {
    setBusy(label);
    setError("");
    setNotice("");
    try {
      return await action();
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy("");
    }
  }
  async function refresh() {
    const list = await call<Connection[]>("browser_connections");
    setConnections(list);
    setConnection((old) =>
      list.some((item) => item.id === old)
        ? old
        : list.length === 1
          ? list[0].id
          : "",
    );
  }
  useEffect(() => {
    if (native) void refresh().catch((error) => setError(String(error)));
  }, []);
  async function capturePage() {
    setCapture(null);
    setPlan(null);
    const value = await operation("Capturing page…", () =>
      call<Capture>("browser_capture", { connectionId: connection }),
    );
    if (value) {
      setCapture(value);
      setSelected(value.page.fields.map((field) => field.id));
    }
  }
  async function loadModels() {
    const result = await operation("Loading models…", () =>
      call<{ data: Model[] }>("codex_models"),
    );
    if (result) {
      const list = result.data.filter((item) => !item.hidden);
      setModels(list);
      setModel(
        list.find((item) => item.isDefault)?.model || list[0]?.model || "",
      );
    }
  }
  async function generate() {
    if (!capture) return;
    setPlan(null);
    const value = await operation("Generating answers…", () =>
      call<Plan>("browser_generate", {
        captureId: capture.id,
        model,
        fieldIds: selected,
      }),
    );
    if (value) setPlan(value);
  }
  async function apply() {
    if (!capture || !plan) return;
    const result = await operation("Filling selected answers…", () =>
      call<{ changed: string[]; error: string }>("browser_apply", {
        captureId: capture.id,
        planId: plan.id,
      }),
    );
    setPlan(null);
    setCapture(null);
    if (result) {
      setNotice(
        `Filled and verified ${result.changed.length} answer${result.changed.length === 1 ? "" : "s"}. The page has not been submitted.`,
      );
      if (result.error) setError(result.error);
    }
  }
  const stage = plan ? 2 : capture ? 1 : 0;
  return (
    <section className="browser-fill" aria-label="Browser page helper">
      <ol className="fill-progress" aria-label="Browser helper progress">
        {["Connect and capture", "Choose and generate", "Review and fill"].map(
          (label, index) => (
            <li key={label} aria-current={stage === index ? "step" : undefined}>
              {label}
            </li>
          ),
        )}
      </ol>
      <div className="section-heading">
        <h2>Current browser page</h2>
        <button
          className="small-button"
          disabled={!native || !!busy}
          onClick={() => void operation("Checking browser…", refresh)}
        >
          Refresh connection
        </button>
      </div>
      {!connections.length ? (
        <div className="fill-setup">
          <p>
            Connect the page you want Buddy to work on using the browser helper.
            Your existing browser session stays open.
          </p>
          <ol>
            <li>
              Load the supplied <strong>browser-helper</strong> folder as an
              unpacked extension in Chrome or Edge.
            </li>
            <li>
              Copy its extension ID and run{" "}
              <strong>Install-Browser-Helper.ps1</strong> from the release
              folder once.
            </li>
            <li>
              Open the question page, click the helper, and choose{" "}
              <strong>Connect this page</strong>. Then refresh here.
            </li>
          </ol>
          <button
            className="small-button"
            disabled={!native || !!busy}
            onClick={() =>
              void operation("Opening helper files…", () =>
                call("browser_helper_files"),
              )
            }
          >
            Open helper files
          </button>
        </div>
      ) : (
        <div className="fill-connection">
          <label>
            Connected page
            <select
              value={connection}
              disabled={!!busy}
              onChange={(event) => {
                setConnection(event.target.value);
                setCapture(null);
                setPlan(null);
              }}
            >
              <option value="">Choose a page</option>
              {connections.map((item) => (
                <option value={item.id} key={item.id}>
                  {item.title} · {item.origin}
                </option>
              ))}
            </select>
          </label>
          <button
            className="small-button"
            disabled={!connection || !!busy}
            onClick={() => void capturePage()}
          >
            Capture questions
          </button>
        </div>
      )}
      {busy && (
        <div className="button-row">
          <p role="status">{busy}</p>
          <button
            className="small-button"
            onClick={() => void call("browser_cancel")}
          >
            Cancel
          </button>
        </div>
      )}
      {error && (
        <p className="message error" role="alert">
          {error}
        </p>
      )}
      {notice && (
        <p className="message notice" role="status">
          {notice}
        </p>
      )}
      {capture && (
        <>
          <h3>{capture.page.title}</h3>
          <p className="path">{capture.page.origin}</p>
          <details className="fill-question" open>
            <summary>Captured question text</summary>
            <pre>{capture.page.question}</pre>
          </details>
          {capture.page.unsupportedEditors > 0 && (
            <p className="message">
              This page includes a code editor that Buddy cannot fill yet. Only
              the supported answer controls below will be used.
            </p>
          )}
          <h3>Answers to include</h3>
          <div className="fill-fields">
            {capture.page.fields.map((field) => (
              <label className="fill-field" key={field.id}>
                <input
                  type="checkbox"
                  checked={selected.includes(field.id)}
                  disabled={!!busy || !!plan}
                  onChange={(event) =>
                    setSelected((old) =>
                      event.target.checked
                        ? [...old, field.id]
                        : old.filter((id) => id !== field.id),
                    )
                  }
                />
                <span>
                  <strong>{field.label}</strong>
                  <span className="muted">
                    {field.kind}
                    {field.options.length
                      ? ` · ${field.options.map((option) => option.label).join(" / ")}`
                      : ""}
                  </span>
                </span>
              </label>
            ))}
          </div>
          {!plan && (
            <div className="fill-generate">
              <label>
                Codex model
                <select
                  value={model}
                  disabled={!!busy}
                  onChange={(event) => setModel(event.target.value)}
                >
                  <option value="">Load your available models</option>
                  {models.map((item) => (
                    <option value={item.model} key={item.model}>
                      {item.displayName || item.model}
                    </option>
                  ))}
                </select>
              </label>
              <div className="button-row">
                <button
                  className="small-button"
                  disabled={!!busy}
                  onClick={() => void loadModels()}
                >
                  Load models
                </button>
                <button
                  className="primary"
                  disabled={!native || !!busy || !model || !selected.length}
                  onClick={() => void generate()}
                >
                  Generate answers
                </button>
              </div>
              <p className="explanation">
                Only the captured text and selected fields are sent to your
                chosen Codex model. This uses your Codex allowance.
              </p>
            </div>
          )}
          {plan && (
            <section className="fill-review">
              <h3>Review proposed answers</h3>
              <p>{plan.explanation}</p>
              {plan.assignments.map((assignment) => {
                const field = capture.page.fields.find(
                  (field) => field.id === assignment.id,
                );
                return (
                  <div className="fill-answer" key={assignment.id}>
                    <strong>{field?.label || assignment.id}</strong>
                    <pre>
                      {field?.options.find(
                        (option) => option.value === assignment.value,
                      )?.label || assignment.value}
                    </pre>
                  </div>
                );
              })}
              <div className="button-row">
                <button
                  className="primary"
                  disabled={!!busy}
                  onClick={() => void apply()}
                >
                  Fill reviewed answers
                </button>
                <button
                  className="small-button"
                  disabled={!!busy}
                  onClick={() => setPlan(null)}
                >
                  Discard answers
                </button>
              </div>
              <p className="explanation">
                Buddy checks that the page and original answers still match.
                Submission stays with you.
              </p>
            </section>
          )}
        </>
      )}
    </section>
  );
}
