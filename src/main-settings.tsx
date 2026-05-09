import { createRoot } from "react-dom/client";
import SettingsRoot from "./Settings";
import "./styles/settings.css";

const root = document.getElementById("root");
if (root) createRoot(root).render(<SettingsRoot />);
