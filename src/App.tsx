import { useEffect, useMemo, useState } from "react";
import { Inbox } from "lucide-react";
import { AppHeader } from "./components/AppHeader";
import { CompactIsland } from "./components/CompactIsland";
import { EventRow } from "./components/EventRow";
import { InterventionPanel } from "./components/InterventionPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { demoState } from "./demo-state";
import { I18nProvider, translator } from "./i18n";
import { bridge } from "./tauri-bridge";
import type { AppState, HookSource, OperationResult } from "./types";

export default function App() {
  const [state, setState] = useState<AppState>(demoState);
  const [selectedId, setSelectedId] = useState<string | undefined>(demoState.selectedEventId ?? demoState.events[0]?.id);
  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    let active = true;
    bridge.getState().then((nextState) => {
      if (!active) return;
      setState(nextState);
      setSelectedId((current) => nextState.selectedEventId ?? current ?? nextState.events[0]?.id);
    });
    const unsubscribe = bridge.onState((nextState) => {
      setState(nextState);
      setSelectedId((current) => nextState.selectedEventId ?? current ?? nextState.events[0]?.id);
    });
    return () => {
      active = false;
      unsubscribe();
    };
  }, []);

  const pendingIDs = useMemo(() => new Set(state.pendingEventIds), [state.pendingEventIds]);
  const selectedEvent = state.events.find((event) => event.id === selectedId) ?? state.events[0];

  async function respond(id: string, response: Parameters<typeof bridge.respond>[1]): Promise<OperationResult> {
    const result = await bridge.respond(id, response);
    if (result.ok && !("__TAURI_INTERNALS__" in window)) {
      setState((current) => ({
        ...current,
        pendingEventIds: current.pendingEventIds.filter((pendingId) => pendingId !== id),
        events: current.events.map((event) => event.id === id
          ? { ...event, resolvedAt: new Date().toISOString(), resolution: response.decision }
          : event),
      }));
    }
    return result;
  }

  async function manageHook(source: HookSource, install: boolean) {
    const result = install ? await bridge.installHooks([source]) : await bridge.uninstallHooks([source]);
    if (result.ok && !("__TAURI_INTERNALS__" in window)) {
      setState((current) => ({
        ...current,
        hookStatus: {
          ...current.hookStatus,
          [source]: { ...current.hookStatus[source]!, installed: install },
        },
      }));
    }
    return result;
  }

  async function updateSettings(patch: Partial<AppState["settings"]>) {
    const settings = await bridge.updateSettings(patch);
    setState((current) => ({ ...current, settings }));
  }

  async function updateSourceProfile(id: HookSource, patch: Partial<Omit<AppState["sourceProfiles"][number], "id">>) {
    const sourceProfiles = await bridge.updateSourceProfile(id, patch);
    setState((current) => ({ ...current, sourceProfiles }));
  }

  async function changeWindowMode(compact: boolean) {
    setSettingsOpen(false);
    setState((current) => ({ ...current, compact }));
    const result = await bridge.setCompact(compact);
    if (!result.ok) setState((current) => ({ ...current, compact: !compact }));
  }

  const pendingEvent = state.events.find((event) => pendingIDs.has(event.id));
  const latestEvent = pendingEvent ?? state.events[0];
  const latestPending = Boolean(pendingEvent);
  const { t } = translator(state.settings.language);

  if (state.compact) {
    return (
      <I18nProvider preference={state.settings.language}>
      <div className="app-shell is-compact">
        <CompactIsland
          event={latestEvent}
          listening={state.listening}
          pending={latestPending}
          profile={state.sourceProfiles.find((profile) => profile.id === latestEvent?.source)}
          onExpand={() => { void changeWindowMode(false); }}
          onSnap={() => { void bridge.snapWindow(); }}
        />
      </div>
      </I18nProvider>
    );
  }

  return (
    <I18nProvider preference={state.settings.language}>
    <div className="app-shell">
      <AppHeader
        listening={state.listening}
        settingsOpen={settingsOpen}
        onToggleSettings={() => setSettingsOpen((open) => !open)}
        onCollapse={() => { void changeWindowMode(true); }}
        onSnap={() => { void bridge.snapWindow(); }}
      />

      {settingsOpen ? (
        <SettingsPanel
          settings={state.settings}
          sourceProfiles={state.sourceProfiles}
          hookStatus={state.hookStatus}
          genericCommand={state.genericCommand}
          version={state.version}
          onUpdateSettings={updateSettings}
          onUpdateSource={updateSourceProfile}
          onInstall={(source) => manageHook(source, true)}
          onUninstall={(source) => manageHook(source, false)}
          onCopyCommand={async () => { await bridge.copyGenericCommand(); }}
          onTest={async () => { await bridge.sendTestEvent(); }}
        />
      ) : state.events.length === 0 ? (
        <main className="empty-state">
          <Inbox size={34} strokeWidth={1.6} />
          <h1>{t("firstNotification")}</h1>
          <p>{t("setupHint")}</p>
          <button className="primary-button" type="button" onClick={() => setSettingsOpen(true)}>{t("openSettings")}</button>
        </main>
      ) : (
        <main className="notification-view">
          {selectedEvent && (pendingIDs.has(selectedEvent.id) || selectedEvent.questions.length > 0) ? (
            <InterventionPanel
              key={selectedEvent.id}
              event={selectedEvent}
              pending={pendingIDs.has(selectedEvent.id)}
              profile={state.sourceProfiles.find((profile) => profile.id === selectedEvent.source)}
              onRespond={(response) => respond(selectedEvent.id, response)}
              onOpenFolder={() => { if (selectedEvent.cwd) bridge.openFolder(selectedEvent.cwd); }}
              onOpenUrl={() => { if (selectedEvent.url) bridge.openUrl(selectedEvent.url); }}
              onOpenTerminal={() => { void bridge.focusEventTerminal(selectedEvent.id); }}
            />
          ) : null}

          <section className="event-list" aria-label={t("recent")}>
            <div className="list-heading"><h2>{t("recent")}</h2><span>{state.events.length}</span></div>
            {state.events.map((event) => (
              <EventRow
                key={event.id}
                event={event}
                selected={event.id === selectedEvent?.id}
                pending={pendingIDs.has(event.id)}
                profile={state.sourceProfiles.find((profile) => profile.id === event.source)}
                onSelect={() => setSelectedId(event.id)}
                onOpenUrl={() => { if (event.url) bridge.openUrl(event.url); }}
                onOpenTerminal={() => { void bridge.focusEventTerminal(event.id); }}
                onDismiss={async () => {
                  await bridge.dismissEvent(event.id);
                  if (!("__TAURI_INTERNALS__" in window)) {
                    setState((current) => ({ ...current, events: current.events.filter((item) => item.id !== event.id) }));
                  }
                }}
              />
            ))}
          </section>
        </main>
      )}
    </div>
    </I18nProvider>
  );
}
