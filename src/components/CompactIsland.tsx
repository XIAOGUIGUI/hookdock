import { ChevronDown, Radio } from "lucide-react";
import type { CSSProperties } from "react";
import { useI18n } from "../i18n";
import type { HookEvent, SourceProfile } from "../types";

interface CompactIslandProps {
  event?: HookEvent;
  listening: boolean;
  pending: boolean;
  profile?: SourceProfile;
  onExpand(): void;
  onSnap(): void;
}

export function CompactIsland({ event, listening, pending, profile, onExpand, onSnap }: CompactIslandProps) {
  const { t } = useI18n();
  const title = event?.title ?? (listening ? t("listening") : t("offline"));

  return (
    <div
      className={`compact-island ${pending ? "has-attention" : ""}`}
      data-tauri-drag-region
      onDoubleClick={onExpand}
      onPointerUp={onSnap}
    >
      <div className="compact-status" aria-label={listening ? t("online") : t("offlineLabel")} style={{ "--source-accent": profile?.accentColor } as CSSProperties}>
        {event ? (
          <><img src="/mascots/hookdock.png" alt="" /><span className="source-mini-badge">{profile?.icon}</span></>
        ) : (
          <Radio size={17} />
        )}
        <span className={`listener-dot ${listening ? "is-online" : ""}`} aria-hidden="true" />
      </div>
      <div className="compact-copy" data-tauri-drag-region>
        <strong>{title}</strong>
        <span>{pending ? t("waiting") : profile?.name ?? event?.providerName ?? t("sources")}</span>
      </div>
      {pending ? <span className="attention-pulse" aria-hidden="true" /> : null}
      <button className="compact-expand" type="button" aria-label={t("expand")} onClick={onExpand}>
        <ChevronDown size={18} strokeWidth={2.3} />
      </button>
    </div>
  );
}
