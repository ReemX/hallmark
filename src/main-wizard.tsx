import { createRoot } from "react-dom/client";
import FirstRunWizardRoot from "./FirstRunWizard";
import "./styles/settings.css";

const root = document.getElementById("root");
if (root) createRoot(root).render(<FirstRunWizardRoot />);
