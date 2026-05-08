import { motion, useReducedMotion } from "framer-motion";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { PopupPayload } from "../types";

const SPRING = { type: "spring" as const, stiffness: 380, damping: 28, mass: 0.9 };

/**
 * The animated pill. Tier driven by className. Accent + halo + tint via CSS.
 * Per CONTEXT.md D-02: icon left, two-line text right (title bold + description).
 * Per CONTEXT.md D-26: when display_name is empty, fall back to ach_api_name.
 */
export function PopupCard({ payload }: { payload: PopupPayload }) {
  const reduceMotion = useReducedMotion();

  // D-26 fallback: api_name as title when display_name is empty.
  const title = payload.display_name && payload.display_name.length > 0
    ? payload.display_name
    : payload.ach_api_name;

  // Icon: Tauri convertFileSrc for local paths (asset:// protocol);
  // direct src for HTTPS Steam CDN URLs (CSP-allowed).
  const iconSrc = payload.icon_path
    ? (payload.icon_path.startsWith("http") ? payload.icon_path : convertFileSrc(payload.icon_path))
    : null;

  return (
    <motion.div
      className={`popup-pill tier-${payload.tier}`}
      initial={reduceMotion ? { opacity: 0 } : { x: 480, opacity: 0 }}
      animate={reduceMotion ? { opacity: 1 } : { x: 0, opacity: 1 }}
      exit={reduceMotion ? { opacity: 0 } : { x: 0, y: -16, opacity: 0 }}
      transition={reduceMotion ? { duration: 0.15 } : SPRING}
    >
      <div className="popup-icon">
        {iconSrc
          ? <img src={iconSrc} alt="" />
          : <div className="popup-icon-placeholder" />
        }
      </div>
      <div className="popup-text">
        <div className="popup-title">{title}</div>
        {payload.description && payload.description.length > 0 && (
          <div className="popup-desc">{payload.description}</div>
        )}
        {payload.global_pct !== null && (
          <div className="popup-rarity">
            {payload.global_pct.toFixed(1)}% of players
          </div>
        )}
      </div>
    </motion.div>
  );
}
