import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { AnimatePresence } from "framer-motion";
import { PopupCard } from "./components/PopupCard";
import type { PopupPayload } from "./types";

function PopupRoot() {
  const [payload, setPayload] = useState<PopupPayload | null>(null);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return; // browser preview — Tauri APIs unavailable
    const unShow = listen<PopupPayload>("popup-show", (e) => setPayload(e.payload));
    const unHide = listen("popup-hide", () => setPayload(null));

    // Phase 4 gap closure (04-09): once both listen() promises resolve,
    // signal the backend that this WebView is ready to receive events.
    // popup_queue::run blocks on this signal before its first emit so
    // the SFX-without-popup race cannot occur on cold-mount in dev or prod.
    Promise.all([unShow, unHide])
      .then(() => invoke("popup_ready"))
      .catch((e) => console.warn("popup_ready invoke failed:", e));

    return () => {
      unShow.then((u) => u());
      unHide.then((u) => u());
    };
  }, []);

  return (
    <AnimatePresence>
      {payload && <PopupCard key={payload.ach_api_name} payload={payload} />}
    </AnimatePresence>
  );
}

const root = document.getElementById("root");
if (root) createRoot(root).render(<PopupRoot />);
