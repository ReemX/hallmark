// First-run wizard entry point — implemented in Plan 04-05.
// Stub to satisfy Vite 4-entry build config.
import { createRoot } from "react-dom/client";

function WizardStub() {
  return <div />;
}

const root = document.getElementById("root");
if (root) createRoot(root).render(<WizardStub />);
