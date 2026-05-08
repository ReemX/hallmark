import { motion } from "framer-motion";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { AchievementSchema } from "../types";

export function AchievementRow({
  ach, earnedAt, expanded, onToggle,
}: {
  ach: AchievementSchema;
  earnedAt: number | null;
  expanded: boolean;
  onToggle: () => void;
}) {
  const isEarned = earnedAt !== null;
  const title = (ach.display_name && ach.display_name.length > 0) ? ach.display_name : ach.ach_api_name;
  const iconSrc = ach.icon_path
    ? (ach.icon_path.startsWith("http") ? ach.icon_path : convertFileSrc(ach.icon_path))
    : null;
  const dateLabel = earnedAt
    ? new Date(earnedAt * 1000).toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" })
    : null;
  return (
    <motion.div
      className={`achievement-row ${isEarned ? "earned" : "locked"} ${expanded ? "expanded" : ""}`}
      role="listitem"
      onClick={onToggle}
      layout
      transition={{ duration: 0.15 }}
    >
      <div className="row-icon">
        {iconSrc
          ? <img src={iconSrc} alt={`${title} achievement icon`} />
          : <div className="row-icon-placeholder" />}
      </div>
      <div className="row-text">
        <div className="row-title">{title}</div>
        {!expanded && ach.description && (
          <div className="row-desc">{ach.description}</div>
        )}
        {expanded && (
          <>
            <div className="row-desc-full">{ach.description ?? ""}</div>
            {dateLabel && <div className="row-meta">Earned {dateLabel}</div>}
          </>
        )}
      </div>
    </motion.div>
  );
}
