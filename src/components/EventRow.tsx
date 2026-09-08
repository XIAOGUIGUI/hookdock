import { Check, CircleAlert, Clock3, ExternalLink, Trash2 } from "lucide-react";
import { useI18n } from "../i18n";
import type { HookEvent, SourceProfile } from "../types";
import { relativeTime } from "../lib/time";

interface EventRowProps {
  event: HookEvent;
  selected: boolean;
  pending: boolean;
  profile?: SourceProfile;
  onSelect(): void;
  onDismiss(): void;
  onOpenUrl(): void;
}

function StatusGlyph({ event }: { event: HookEvent }) {
  if (event.resolvedAt || event.status === "completed") return <Check size={14} />;
  if (event.status === "waitingApproval" || event.status === "waitingInput" || event.status === "error") {
    return <CircleAlert size={14} />;
  }
  return <Clock3 size={14} />;
}

export function EventRow({ event, selected, pending, profile, onSelect, onDismiss, onOpenUrl }: EventRowProps) {
  const { locale, t } = useI18n();
  return (
    <article className={`event-row ${selected ? "is-selected" : ""}`}>
      <button className="event-select" type="button" onClick={onSelect}>
        <span className="source-avatar" style={{ borderColor: profile?.accentColor, color: profile?.accentColor }} aria-hidden="true">
          <img className="mascot" src="/mascots/hookdock.png" alt="" />
          <span>{profile?.icon ?? "⚓"}</span>
        </span>
        <span className="event-copy">
          <span className="event-title-line">
            <strong>{event.title}</strong>
            {pending ? <span className="pending-label">{t("pending")}</span> : null}
          </span>
          <span className="event-body">{event.body}</span>
          <span className="event-meta">
            {profile?.name ?? event.providerName}<span aria-hidden="true">·</span>{event.project}<span aria-hidden="true">·</span><StatusGlyph event={event} />{relativeTime(event.receivedAt, locale)}
          </span>
        </span>
      </button>
      <span className="event-actions">
        {event.url ? <button className="dismiss-button" type="button" aria-label={t("openLink")} onClick={onOpenUrl}><ExternalLink size={15} /></button> : null}
        {!pending ? <button className="dismiss-button" type="button" aria-label={`${t("remove")} ${event.title}`} onClick={onDismiss}><Trash2 size={15} /></button> : null}
      </span>
    </article>
  );
}
