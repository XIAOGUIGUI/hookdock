import { useState } from "react";
import { Check, ExternalLink, FolderOpen, SquareTerminal, X } from "lucide-react";
import { useI18n } from "../i18n";
import type { HookEvent, HookResponse, OperationResult, SourceProfile } from "../types";

interface InterventionPanelProps {
  event: HookEvent;
  pending: boolean;
  profile?: SourceProfile;
  onRespond(response: HookResponse): Promise<OperationResult>;
  onOpenFolder(): void;
  onOpenUrl(): void;
  onOpenTerminal(): void;
}

export function InterventionPanel({ event, pending, profile, onRespond, onOpenFolder, onOpenUrl, onOpenTerminal }: InterventionPanelProps) {
  const { t } = useI18n();
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const allQuestionsAnswered = event.questions.every((question) => (answers[question.id] ?? "").trim().length > 0);

  async function submit(response: HookResponse) {
    setBusy(true);
    setError(undefined);
    const result = await onRespond(response);
    if (!result.ok) setError(result.error ?? t("submitFailed"));
    setBusy(false);
  }

  function toggleOption(questionId: string, label: string, multiSelect: boolean) {
    setAnswers((current) => {
      if (!multiSelect) return { ...current, [questionId]: label };
      const selected = new Set((current[questionId] ?? "").split(", ").filter(Boolean));
      if (selected.has(label)) selected.delete(label);
      else selected.add(label);
      return { ...current, [questionId]: [...selected].join(", ") };
    });
  }

  return (
    <section className="intervention-panel" aria-label={t("details")}>
      <div className="intervention-heading">
        <div>
          <p className="eyeline">{profile?.name ?? event.providerName} · {event.project}</p>
          <h1>{event.title}</h1>
        </div>
        <div className="detail-actions">
          {event.terminalTarget ? <button className="icon-button subtle" type="button" aria-label={t("openTerminal")} onClick={onOpenTerminal}><SquareTerminal size={18} /></button> : null}
          {event.url ? <button className="icon-button subtle" type="button" aria-label={t("openLink")} onClick={onOpenUrl}><ExternalLink size={18} /></button> : null}
          {event.cwd ? <button className="icon-button subtle" type="button" aria-label={t("openFolder")} onClick={onOpenFolder}><FolderOpen size={18} /></button> : null}
        </div>
      </div>
      <p className="intervention-message">{event.body}</p>

      {event.questions.map((question) => (
        <fieldset className="question" key={question.id} disabled={!pending || busy}>
          <legend>{question.prompt}</legend>
          {question.options.length > 0 ? (
            <div className="question-options">
              {question.options.map((option) => {
                const selected = (answers[question.id] ?? "").split(", ").includes(option.label);
                return (
                  <button
                    className={`option-button ${selected ? "is-selected" : ""}`}
                    type="button"
                    key={option.id}
                    onClick={() => toggleOption(question.id, option.label, question.multiSelect)}
                  >
                    <span>{option.label}</span>
                    {option.description ? <small>{option.description}</small> : null}
                  </button>
                );
              })}
            </div>
          ) : (
            <input
              className="answer-input"
              value={answers[question.id] ?? ""}
              onChange={(inputEvent) => setAnswers((current) => ({ ...current, [question.id]: inputEvent.target.value }))}
              placeholder={t("answerPlaceholder")}
            />
          )}
        </fieldset>
      ))}

      {pending && event.questions.length > 0 ? (
        <div className="intervention-actions">
          <button className="primary-button" type="button" disabled={busy || !allQuestionsAnswered} onClick={() => submit({ decision: "answer", answers })}>
            <Check size={17} />{t("submitAnswer")}
          </button>
          <button className="danger-button" type="button" disabled={busy} onClick={() => submit({ decision: "deny" })}>
            <X size={17} />{t("cancel")}
          </button>
        </div>
      ) : null}

      {pending && event.questions.length === 0 ? (
        <div className="intervention-actions">
          <button className="primary-button" type="button" disabled={busy} onClick={() => submit({ decision: "approve" })}>
            <Check size={17} />{t("allowOnce")}
          </button>
          <button className="danger-button" type="button" disabled={busy} onClick={() => submit({ decision: "deny" })}>
            <X size={17} />{t("deny")}
          </button>
        </div>
      ) : null}

      {!pending && event.questions.length > 0 && !event.resolution ? (
        <p className="terminal-handoff-note">{t("terminalHandoff")}</p>
      ) : null}

      {!pending && event.resolution ? <p className="resolution-note">{t("handled")}: {event.resolution}</p> : null}
      {error ? <p className="inline-error" role="alert">{error}</p> : null}
    </section>
  );
}
