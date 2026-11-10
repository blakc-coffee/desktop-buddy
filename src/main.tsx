import React, { lazy, Suspense } from "react";
import { createRoot } from "react-dom/client";
import "./style.css";
const surface =
  new URLSearchParams(location.search).get("surface") ||
  (window as unknown as { __BUDDY_SURFACE__?: string }).__BUDDY_SURFACE__;
const View =
  surface === "todo"
    ? lazy(() => import("./Todo"))
    : surface === "preview"
      ? lazy(() => import("./HeldPreview"))
      : surface === "explanation"
        ? lazy(() => import("./Explanation"))
        : lazy(() => import("./App"));
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Suspense fallback={null}>
      <View />
    </Suspense>
  </React.StrictMode>,
);
