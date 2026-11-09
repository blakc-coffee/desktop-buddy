import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { call, native } from "./bridge";
import type { Snapshot } from "./domain";
export type ScreenResult = {
  title?: string;
  question?: string;
  busy?: boolean;
  streaming?: boolean;
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
export function BubbleClose() {
  useEffect(() => {
    const close = (e: KeyboardEvent) => {
      if (e.key === "Escape" && native) void call("bubble_dismiss");
    };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, []);
  return (
    <button
      className="bubble-close"
      aria-label="Close bubble"
      onClick={() => {
        if (native) void call("bubble_dismiss");
      }}
    >
      ×
    </button>
  );
}
export default function Explanation() {
  const [result, setResult] = useState<ScreenResult | null>(null);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState("");
  const [followup, setFollowup] = useState("");
  const [sending, setSending] = useState(false);
  const bubble = useRef<HTMLElement>(null);
  const ready =
    !!result &&
    (!result.busy || result.streaming) &&
    !!(result.explanation || result.error || result.typingError);
  useLayoutEffect(() => {
    if (!ready || !bubble.current) return;
    const element = bubble.current;
    let alive = true,
      frame = 0,
      last = "";
    const measure = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const text =
          result?.explanation || result?.error || result?.typingError || "";
        const width = text.includes("```")
          ? 380
          : Math.min(
              380,
              Math.max(300, Math.ceil(Math.sqrt(text.length * 170)) + 100),
            );
        const height = Math.max(
          100,
          Math.min(
            360,
            screen.availHeight - 24,
            element.scrollHeight +
              26 +
              (element.querySelector<HTMLElement>(".bubble-answer")
                ?.scrollHeight || 0) -
              (element.querySelector<HTMLElement>(".bubble-answer")
                ?.clientHeight || 0),
          ),
        );
        const key = `${width}:${height}`;
        if (key === last) return;
        last = key;
        if (native)
          void call<{ side: string; tail: number }>("bubble_present", {
            width,
            height,
          })
            .then((anchor) => {
              if (!alive || !anchor) return;
              document.documentElement.dataset.tail = anchor.side;
              document.documentElement.style.setProperty(
                "--bubble-tail",
                `${anchor.tail}%`,
              );
            })
            .catch(() => {});
      });
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    measure();
    const outside = (event: PointerEvent) => {
      if (!element.contains(event.target as Node) && native)
        void call("bubble_dismiss");
    };
    document.addEventListener("pointerdown", outside);
    return () => {
      alive = false;
      cancelAnimationFrame(frame);
      observer.disconnect();
      document.removeEventListener("pointerdown", outside);
    };
  }, [ready, result, copied]);
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
  if (!ready) return null;
  return (
    <main ref={bubble} className="explanation-bubble">
      <BubbleClose />
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
        {result?.explanation && (
          <button
            className="bubble-copy"
            aria-label="Copy explanation"
            title="Copy explanation"
            onClick={() => void copy(result.explanation, "Copied")}
          >
            <svg
              aria-hidden="true"
              viewBox="0 0 24 24"
              width="17"
              height="17"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.6"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <rect x="8" y="8" width="12" height="13" rx="2" />
              <path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3" />
            </svg>
          </button>
        )}
      </header>
      <div
        className="bubble-answer"
        role="region"
        aria-label="Answer"
        tabIndex={0}
      >
        {error && <p role="alert">{error}</p>}

        {result?.error && <p role="alert">{result.error}</p>}
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
          </>
        ) : !result?.busy && !result?.error ? (
          <p>
            No explanation yet. Use /exp to capture and explain your screen.
          </p>
        ) : null}
      </div>
      <form
        className="bubble-followup"
        onSubmit={async (event) => {
          event.preventDefault();
          if (!followup.trim() || sending || result?.busy) return;
          setSending(true);
          setError("");
          try {
            await call("bubble_followup", { question: followup.trim() });
            setFollowup("");
          } catch (error) {
            setError(String(error));
          } finally {
            setSending(false);
          }
        }}
      >
        <input
          aria-label="Follow-up question"
          placeholder="Ask a follow-up…"
          value={followup}
          maxLength={1000}
          onChange={(event) => setFollowup(event.target.value)}
          onFocus={() => {
            if (native) void call("ai_prepare");
          }}
          disabled={sending || result?.busy}
        />
        <button
          type="submit"
          className="followup-send"
          aria-label="Send follow-up"
          title="Send follow-up"
          disabled={sending || result?.busy || !followup.trim()}
        >
          <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            width="17"
            height="17"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.7"
          >
            <path d="m5 12 7-7 7 7M12 5v15" />
          </svg>
        </button>
      </form>
      <p className="copy-status" role="status">
        {copied}
      </p>
    </main>
  );
}
