import { createRoot } from "react-dom/client";

function CompanionStub() {
  return <div style={{ color: "#F0F0F5", fontFamily: "system-ui" }}>Hallmark companion ready (Plan 06 will populate).</div>;
}
const root = document.getElementById("root");
if (root) createRoot(root).render(<CompanionStub />);
