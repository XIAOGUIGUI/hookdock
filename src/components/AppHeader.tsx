import { ChevronUp, Settings } from "lucide-react";
import { useI18n } from "../i18n";

interface AppHeaderProps {
  listening: boolean;
  settingsOpen: boolean;
  onToggleSettings(): void;
  onCollapse(): void;
  onSnap(): void;
}

export function AppHeader({ listening, settingsOpen, onToggleSettings, onCollapse, onSnap }: AppHeaderProps) {
  const { t } = useI18n();
  return (
    <header className="app-header" data-tauri-drag-region onPointerUp={onSnap}>
      <div className="brand-lockup">
        <img className="island-mark" src="/mascots/hookdock.png" alt="" />
        <span className={`listener-dot ${listening ? "is-online" : ""}`} aria-hidden="true" />
        <div>
          <p className="listener-title">{listening ? t("listening") : t("offline")}</p>
          <p className="listener-caption">{t("sources")}</p>
        </div>
      </div>
      <div className="window-actions">
        <button
          className={`icon-button ${settingsOpen ? "is-active" : ""}`}
          type="button"
          aria-label={t("settings")}
          onClick={onToggleSettings}
        >
          <Settings size={19} strokeWidth={2.2} />
        </button>
        <button className="icon-button" type="button" aria-label={t("collapse")} onClick={onCollapse}>
          <ChevronUp size={20} strokeWidth={2.2} />
        </button>
      </div>
    </header>
  );
}
