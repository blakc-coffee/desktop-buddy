import { useEffect, useState } from "react";
import { call, native } from "./bridge";
import type { Snapshot } from "./domain";
export type ScreenResult = {
  title?: string;
  question?: string;
  busy?: boolean;
  error?: string;
  explanation: string;
  answer?: string;
  typed?: boolean;
  typingError?: string;
};
export function ResultText({ text }: { text: string }) {
  // Render fenced code as text, never executable HTML or model-supplied markup.
  return (
    <div className="explanation-content">
      {text.split(/```[^\n]*\n([\s\S]*?)```/g).map((part, index) =>
        index % 2 ? (
          <pre key={index}>
            <code>{part}</code>
          </pre>
        ) : (
          <p key={index}>{part}</p>
        ),
      )}
    </div>
  );
}
export default function Explanation() {
  const [result, setResult] = useState<ScreenResult | null>(null);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState("");
  useEffect(() => {
    let alive = true;
    let stop = () => {};
    async function refresh() {
      try {
        const value = await call<ScreenResult>("screen_result");
        if (alive) setResult(value);
      } catch (error) {
        if (alive) setError(String(error));
      }
    }
    void call<Snapshot>("snapshot")
      .then((snapshot) => {
        if (!alive) return;
        document.documentElement.dataset.theme = snapshot.theme;
        document.documentElement.dataset.pet =
          snapshot.petAppearance || "calico";
      })
      .catch(() => {});
    if (native)
      void import("@tauri-apps/api/event").then(async ({ listen }) => {
        const off = await listen("screen-result", () => void refresh());
        if (alive) {
          stop = off;
          void refresh();
        } else off();
      });
    else void refresh();
    return () => {
      alive = false;
      stop();
    };
  }, []);
  async function copy(text: string, label: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(label);
    } catch (error) {
      setError(String(error));
    }
  }
  return (
    <main className="explanation-bubble">
      <header>
        <svg
          aria-hidden="true"
          viewBox="0 0 24 24"
          width="26"
          height="26"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
        >
          <path d="M4 10 3 3l6 4h6l6-4-1 7v6c-3 6-13 6-16 0zM7 13h2m6 0h2m-6 3h2m-8-1H1m18 0h4" />
        </svg>
        <h1>{result?.title || "Explanation"}</h1>
      </header>
      {error && <p role="alert">{error}</p>}
      {result?.question && <p className="quick-question">{result.question}</p>}
      {result?.error && <p role="alert">{result.error}</p>}
      {result?.busy && (
        <div className="screen-status" role="status">
          <span>Asking your AI…</span>
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
        </div>
      )}
      {result?.typingError && <p role="status">{result.typingError}</p>}
      {result?.explanation ? (
        <>
          <ResultText text={result.explanation} />
          {result.answer && (
            <section className="recoverable-answer">
              <h2>Answer</h2>
              <pre>{result.answer}</pre>
              <button
                className="small-button"
                onClick={() => void copy(result.answer!, "Answer copied")}
              >
                Copy answer
              </button>
            </section>
          )}
          <button
            className="small-button"
            onClick={() => void copy(result.explanation, "Explanation copied")}
          >
            Copy explanation
          </button>
        </>
      ) : !result?.busy && !result?.error ? (
        <p>No explanation yet. Use /exp to capture and explain your screen.</p>
      ) : null}
      <p className="copy-status" role="status">
        {copied}
      </p>
    </main>
  );
}
