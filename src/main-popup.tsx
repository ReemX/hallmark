import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { AnimatePresence } from "framer-motion";
import { PopupCard } from "./components/PopupCard";
import type { PopupPayload } from "./types";

function PopupRoot() {
  const [payload, setPayload] = useState<PopupPayload | null>(null);

  useEffect(() => {
    const unShow = listen<PopupPayload>("popup-show", (e) => setPayload(e.payload));
    const unHide = listen("popup-hide", () => setPayload(null));
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
