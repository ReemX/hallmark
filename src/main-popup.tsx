import { createRoot } from "react-dom/client";

// Plan 05 replaces this stub with PopupRoot (Framer Motion AnimatePresence).
function PopupStub() {
  return <div style={{ color: "#fff", fontFamily: "system-ui" }}>Hallmark popup ready (Plan 05 will populate).</div>;
}
const root = document.getElementById("root");
if (root) createRoot(root).render(<PopupStub />);
