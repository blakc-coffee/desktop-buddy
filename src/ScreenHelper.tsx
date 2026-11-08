import { lazy, Suspense, useState, useEffect, useRef } from "react";
import type { PanelRequest } from "./panelNavigation";
import { call, native } from "./bridge";
const BrowserFill = lazy(() => import("./BrowserFill"));
type Capture = {
  id: string;
  preview: string;
  width: number;
  height: number;
  target: string;
};
type Model = {
  model: string;
  displayName: string;
  isDefault: boolean;
  hidden?: boolean;
};
export default function ScreenHelper({
  navigation,
  visible,
  onNavigated,
}: {
  navigation: PanelRequest | null;
  visible: boolean;
  onNavigated: (id: number) => void;
}) {
  const [capture, setCapture] = useState<Capture | null>(null);
  const [question, setQuestion] = useState("");
  const [models, setModels] = useState<Model[]>([]);
  const [model, setModel] = useState(
    () => localStorage.getItem("buddy-screen-model") || "",
  );
  const [pending, setPending] = useState<"explain" | "fill" | null>(null);
  const [typed, setTyped] = useState(false);
  const seen = useRef(0);
  const [answer, setAnswer] = useState("");
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [browser, setBrowser] = useState(false);
  async function run<T>(
    label: string,
    operation: () => Promise<T>,
  ): Promise<T | undefined> {
    setBusy(label);
    setError("");
    try {
      return await operation();
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy("");
    }
  }
  async function takeCapture(mode?: "explain" | "fill") {
    setPending(null);
    setTyped(false);
    setCapture(null);
    setAnswer("");
    const result = await run("Capturing in 3 seconds…", () =>
      call<Capture>("screen_capture"),
    );
    if (result) {
      setCapture(result);
      if (mode) setPending(mode);
    }
  }
  async function loadModels() {
    const result = await run("Loading models…", () =>
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
  async function explain(fill = false) {
    if (!capture) return;
    setAnswer("");
    const result = await run("Explaining your capture…", () =>
      call<{ explanation: string; typed?: boolean; typingError?: string }>(
        "screen_explain",
        {
          captureId: capture.id,
          question,
          model,
          fill,
        },
      ),
    );
    if (result) {
      setAnswer(result.explanation);
      setTyped(!!result.typed);
      if (result.typingError) setError(result.typingError);
    }
  }
  async function clear() {
    const result = await run("Clearing capture…", async () => {
      await call("screen_clear");
      return true;
    });
    if (result) {
      setPending(null);
      setCapture(null);
      setAnswer("");
      setTyped(false);
    }
  }
  useEffect(() => {
    if (model) localStorage.setItem("buddy-screen-model", model);
  }, [model]);
  useEffect(() => {
    if (
      !visible ||
      busy ||
      !navigation ||
      navigation.id <= seen.current ||
      !["screenCapture", "screenExplain"].includes(navigation.destination)
    )
      return;
    seen.current = navigation.id;
    onNavigated(navigation.id);
    void takeCapture(
      navigation.destination === "screenCapture" ? "fill" : "explain",
    );
  }, [navigation?.id, visible, busy]);
  useEffect(() => {
    if (pending && capture && model && !busy) {
      const mode = pending;
      setPending(null);
      void explain(mode === "fill");
    }
  }, [pending, capture, model, busy]);
  return (
    <section className="screen-helper" aria-label="Screen explanation helper">
      <div className="screen-capture-intro">
        <div>
          <h2>Understand what’s on your screen</h2>
          <p>Questions, code, diagrams, documents or errors in any app.</p>
        </div>
        <button
          className="primary"
          disabled={!native || !!busy}
          onClick={() => void takeCapture()}
        >
          {capture ? "Capture again" : "Capture screen"}
        </button>
      </div>
      <p className="muted">
        Buddy hides for 3 seconds. Show the content you want and move your
        pointer onto its display. Capture screen lets you preview first. /exp
        captures and explains; /sc captures and types an answer into the app
        active at capture time. Focus its answer field during the countdown.
        Commands send the capture using your saved model.
      </p>
      {!native && <p>Screen capture is available in the Windows app.</p>}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {busy && (
        <div className="screen-status" role="status">
          <span>{busy}</span>
          {busy !== "Capturing in 3 seconds…" && (
            <button
              className="small-button"
              onClick={() =>
                void call("screen_cancel").catch((error) =>
                  setError(String(error)),
                )
              }
            >
              Cancel
            </button>
          )}
        </div>
      )}
      {capture && (
        <>
          <figure className="screen-preview">
            <img
              src={capture.preview}
              alt="Captured display to review before explaining"
              width={capture.width}
              height={capture.height}
            />
            <figcaption>
              {capture.width} × {capture.height} ·{" "}
              {capture.target || "Captured display"}
            </figcaption>
          </figure>
          <label className="screen-question">
            What would you like to understand?{" "}
            <span className="muted">Optional</span>
            <textarea
              maxLength={4000}
              rows={3}
              value={question}
              disabled={!!busy}
              onChange={(event) => setQuestion(event.target.value)}
              placeholder="Explain this question, walk through the code, or help me understand the diagram…"
            />
          </label>
          <div className="screen-models">
            <button
              className="small-button"
              disabled={!!busy}
              onClick={() => void loadModels()}
            >
              {models.length ? "Refresh models" : "Load models"}
            </button>
            {(models.length > 0 || model) && (
              <label>
                Model
                <select
                  value={model}
                  disabled={!!busy}
                  onChange={(event) => setModel(event.target.value)}
                >
                  {model && !models.some((item) => item.model === model) && (
                    <option value={model}>{model}</option>
                  )}
                  {models.map((item) => (
                    <option key={item.model} value={item.model}>
                      {item.displayName || item.model}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>
          {pending && !model && (
            <p role="status">
              Load and select a model once to continue this command. Buddy
              remembers your choice.
            </p>
          )}
          <p className="muted">
            Explain sends this image and your question to the selected model
            through your signed-in Codex CLI. Check the preview for private
            content. /sc types text only; it does not press Enter, click choices
            or submit. Text entry depends on the target app and cannot be
            verified for every control.
          </p>
          <div className="row-actions">
            <button
              className="primary"
              disabled={!!busy || !model}
              onClick={() => void explain(false)}
            >
              Explain capture
            </button>
            <button
              className="small-button"
              disabled={!!busy}
              onClick={() => void clear()}
            >
              Clear capture
            </button>
          </div>
        </>
      )}
      {typed && (
        <p role="status">
          Answer text sent to the captured app. Check its field; Buddy has not
          submitted it.
        </p>
      )}
      {answer && (
        <section className="screen-answer" aria-label="Screen explanation">
          <div className="section-heading">
            <h2>Explanation</h2>
            <button
              className="small-button"
              onClick={() =>
                void navigator.clipboard
                  .writeText(answer)
                  .catch((error) => setError(String(error)))
              }
            >
              Copy explanation
            </button>
          </div>
          <div className="screen-answer-text">{answer}</div>
        </section>
      )}
      <details
        className="screen-browser-options"
        onToggle={(event) => {
          if (event.currentTarget.open) setBrowser(true);
        }}
      >
        <summary>Optional: fill answers in a browser page</summary>
        <p>
          The screen helper works without an extension. Browser filling uses the
          separate page connection and lets you review answers before applying
          them.
        </p>
        {browser && (
          <Suspense fallback={<p role="status">Loading browser tools…</p>}>
            <BrowserFill />
          </Suspense>
        )}
      </details>
    </section>
  );
}
