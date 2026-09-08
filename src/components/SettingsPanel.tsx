import { useState, type CSSProperties } from "react";
import { BellRing, Clipboard, FlaskConical, Languages, PlugZap, Power } from "lucide-react";
import { useI18n } from "../i18n";
import type {
  AppSettings,
  HookInstallState,
  HookSource,
  OperationResult,
  SourceProfile,
} from "../types";

interface SettingsPanelProps {
  settings: AppSettings;
  sourceProfiles: SourceProfile[];
  hookStatus: Partial<Record<HookSource, HookInstallState>>;
  genericCommand: string;
  version: string;
  onUpdateSettings(patch: Partial<AppSettings>): Promise<void>;
  onUpdateSource(id: HookSource, patch: Partial<Omit<SourceProfile, "id">>): Promise<void>;
  onInstall(source: HookSource): Promise<OperationResult>;
  onUninstall(source: HookSource): Promise<OperationResult>;
  onCopyCommand(): Promise<void>;
  onTest(): Promise<void>;
}

const hookSources: HookSource[] = ["claude", "codex", "gemini"];

function Toggle({ checked, onChange, label }: { checked: boolean; onChange(value: boolean): void; label: string }) {
  return (
    <button className={`toggle ${checked ? "is-on" : ""}`} type="button" role="switch" aria-checked={checked} aria-label={label} onClick={() => onChange(!checked)}>
      <span />
    </button>
  );
}

function SourceCard({ profile, onUpdate }: { profile: SourceProfile; onUpdate(patch: Partial<Omit<SourceProfile, "id">>): Promise<void> }) {
  const { t } = useI18n();
  return (
    <article className="source-card" style={{ "--profile-accent": profile.accentColor } as CSSProperties}>
      <header>
        <span className="profile-glyph">{profile.icon}</span>
        <strong>{profile.name}</strong>
        <Toggle checked={profile.enabled} label={`${profile.name} ${t("enabled")}`} onChange={(enabled) => { void onUpdate({ enabled }); }} />
      </header>
      <div className="profile-fields">
        <label><span>{t("name")}</span><input defaultValue={profile.name} maxLength={40} onBlur={(event) => { if (event.target.value !== profile.name) void onUpdate({ name: event.target.value }); }} /></label>
        <label className="icon-field"><span>{t("icon")}</span><input defaultValue={profile.icon} maxLength={8} onBlur={(event) => { if (event.target.value !== profile.icon) void onUpdate({ icon: event.target.value }); }} /></label>
        <label className="color-field"><span>{t("color")}</span><input type="color" value={profile.accentColor} onChange={(event) => { void onUpdate({ accentColor: event.target.value }); }} /></label>
      </div>
      <div className="profile-toggles">
        <label>{t("toast")}<Toggle checked={profile.showToast} label={`${profile.name} ${t("toast")}`} onChange={(showToast) => { void onUpdate({ showToast }); }} /></label>
        <label>{t("sound")}<Toggle checked={profile.playSound} label={`${profile.name} ${t("sound")}`} onChange={(playSound) => { void onUpdate({ playSound }); }} /></label>
        <label>{t("sourceExpand")}<Toggle checked={profile.autoExpand} label={`${profile.name} ${t("sourceExpand")}`} onChange={(autoExpand) => { void onUpdate({ autoExpand }); }} /></label>
      </div>
    </article>
  );
}

export function SettingsPanel(props: SettingsPanelProps) {
  const { t } = useI18n();
  const [message, setMessage] = useState<string>();

  async function manage(source: HookSource, installed: boolean) {
    setMessage(undefined);
    const result = installed ? await props.onUninstall(source) : await props.onInstall(source);
    setMessage(result.ok ? `${props.hookStatus[source]?.title ?? source}: ${installed ? t("disconnect") : t("connect")}` : result.error);
  }

  return (
    <main className="settings-panel">
      <div className="settings-title">
        <div>
          <p className="eyeline">HookDock for Windows</p>
          <h1>{t("notificationHub")}</h1>
        </div>
        <span className="version">v{props.version}</span>
      </div>

      <section className="setting-group">
        <div className="setting-row">
          <BellRing size={19} />
          <div><strong>{t("windowsToasts")}</strong><small>{t("windowsToastsHint")}</small></div>
          <Toggle label={t("windowsToasts")} checked={props.settings.notificationsEnabled} onChange={(notificationsEnabled) => { void props.onUpdateSettings({ notificationsEnabled }); }} />
        </div>
        <div className="setting-row">
          <PlugZap size={19} />
          <div><strong>{t("autoExpand")}</strong><small>{t("autoExpandHint")}</small></div>
          <Toggle label={t("autoExpand")} checked={props.settings.showPanelOnAttention} onChange={(showPanelOnAttention) => { void props.onUpdateSettings({ showPanelOnAttention }); }} />
        </div>
        <div className="setting-row">
          <Power size={19} />
          <div><strong>{t("launchAtLogin")}</strong><small>{t("launchAtLoginHint")}</small></div>
          <Toggle label={t("launchAtLogin")} checked={props.settings.openAtLogin} onChange={(openAtLogin) => { void props.onUpdateSettings({ openAtLogin }); }} />
        </div>
        <div className="setting-row language-row">
          <Languages size={19} />
          <div><strong>{t("language")}</strong><small>{t("languageHint")}</small></div>
          <select value={props.settings.language} aria-label={t("language")} onChange={(event) => { void props.onUpdateSettings({ language: event.target.value as AppSettings["language"] }); }}>
            <option value="auto">{t("automatic")}</option>
            <option value="zh-CN">{t("chinese")}</option>
            <option value="en">{t("english")}</option>
          </select>
        </div>
      </section>

      <section className="hook-settings">
        <h2>{t("hookConnections")}</h2>
        {hookSources.map((source) => {
          const status = props.hookStatus[source];
          const profile = props.sourceProfiles.find((candidate) => candidate.id === source);
          return (
            <div className="hook-row" key={source}>
              <span className="hook-glyph" style={{ color: profile?.accentColor }}>{profile?.icon ?? "•"}</span>
              <div><strong>{status?.title ?? profile?.name ?? source}</strong><small>{status?.installed ? t("connected") : t("disconnected")}</small></div>
              <button className={status?.installed ? "text-button danger-text" : "text-button"} type="button" onClick={() => { void manage(source, status?.installed === true); }}>
                {status?.installed ? t("disconnect") : t("connect")}
              </button>
            </div>
          );
        })}
      </section>

      <section className="profile-section">
        <h2>{t("sourceProfiles")}</h2>
        {props.sourceProfiles.map((profile) => <SourceCard key={profile.id} profile={profile} onUpdate={(patch) => props.onUpdateSource(profile.id, patch)} />)}
      </section>

      <section className="generic-hook">
        <div><strong>{t("customHook")}</strong><small>{props.genericCommand || t("customHookHint")}</small></div>
        <button className="icon-button subtle" type="button" aria-label={t("copyCommand")} onClick={() => { void props.onCopyCommand(); }}>
          <Clipboard size={17} />
        </button>
      </section>

      <button className="test-button" type="button" onClick={() => { void props.onTest(); }}>
        <FlaskConical size={17} />{t("sendTest")}
      </button>
      {message ? <p className="settings-message" role="status">{message}</p> : null}
    </main>
  );
}
