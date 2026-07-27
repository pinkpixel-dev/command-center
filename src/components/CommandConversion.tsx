import { ArrowRight, Copy, FilePlus2, Repeat2, X } from "lucide-react";

import { useShellConversion } from "../hooks/useShellConversion";
import type { CommandProposal, Equivalence, ShellConversion } from "../lib/types";
import { Button } from "./ui/Button";
import { RiskBadge } from "./ui/RiskBadge";

export interface CommandConversionProps {
  commandId: number;
  /** The shell the entry is saved as, used to leave it out of the picker. */
  entryShell: string | null;
  /** AI is on, a key is stored, and the credential manager answered. */
  ready: boolean;
  onCopy: (text: string) => void;
  onReview: (proposal: CommandProposal) => void;
}

/**
 * How much of the original behaviour survived. None of these claim the
 * conversion is exact, because none of them can: two shells that print the
 * same output can still differ on globbing, quoting, or exit status.
 */
const EQUIVALENCE_NOTES: Record<Equivalence, string> = {
  close: "Does the same job on a normal machine. Check the differences before you rely on it.",
  partial: "Some of the original behaviour did not carry over. Read what changed.",
  uncertain: "This rewrite is a best effort and may not be right. Test it before you use it.",
};

/**
 * The Convert section inside the full entry dialog, beside Explain. It rewrites
 * one saved entry for another shell and shows both, along with what changed
 * underneath. Nothing is saved from here: Review and save opens the normal
 * entry form, so a conversion never overwrites the original.
 */
export function CommandConversion({
  commandId,
  entryShell,
  ready,
  onCopy,
  onReview,
}: CommandConversionProps) {
  const { shells, result, converting, error, cancelled, convert, cancel, reset } =
    useShellConversion(commandId, ready);

  if (!ready) return null;

  const saved = entryShell?.trim().toLowerCase() ?? null;
  const targets = shells.filter((shell) => shell.id !== saved);

  return (
    <section className="convert" aria-labelledby={`convert-heading-${commandId}`}>
      <div className="convert__head">
        <p className="card__section-label" id={`convert-heading-${commandId}`}>
          Convert to another shell
        </p>
        {result && (
          <Button variant="ghost" size="sm" onClick={reset}>
            <Repeat2 size={14} aria-hidden="true" />
            Try another shell
          </Button>
        )}
      </div>

      {!result && (
        <>
          <p className="explain__note">
            Rewrite this entry for a different shell. You get the rewrite and what changes with it,
            never a promise that the two behave identically.
          </p>

          <div className="convert__targets" role="group" aria-label="Convert to">
            {targets.map((shell) => (
              <Button
                key={shell.id}
                variant="secondary"
                size="sm"
                disabled={converting}
                onClick={() => convert(shell.id)}
              >
                {shell.label}
              </Button>
            ))}
            {converting && (
              <Button variant="ghost" size="sm" onClick={cancel}>
                <X size={14} aria-hidden="true" />
                Stop
              </Button>
            )}
          </div>

          {converting && <p className="explain__note">Rewriting. This can take a moment.</p>}
        </>
      )}

      {result && <ConversionResult conversion={result} onCopy={onCopy} onReview={onReview} />}

      {cancelled && (
        <p className="assistant__note assistant__note--stopped" role="status">
          Stopped. Nothing was converted.
        </p>
      )}

      {error && (
        <p className="explain__error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}

interface ConversionResultProps {
  conversion: ShellConversion;
  onCopy: (text: string) => void;
  onReview: (proposal: CommandProposal) => void;
}

function ConversionResult({ conversion, onCopy, onReview }: ConversionResultProps) {
  const { converted } = conversion;

  return (
    <div className="convert__result">
      <p className="convert__shells">
        <span className="convert__shell">{conversion.sourceShell ?? "unspecified"}</span>
        <ArrowRight size={13} aria-hidden="true" />
        <span className="convert__shell convert__shell--target">{conversion.targetShell}</span>
      </p>

      <div className="convert__pair">
        <div className="convert__side">
          <p className="card__section-label">Original</p>
          <pre className="code convert__code" tabIndex={0}>
            <code>{conversion.original}</code>
          </pre>
        </div>
        <div className="convert__side">
          <p className="card__section-label">Converted</p>
          <pre className="code convert__code convert__code--target" tabIndex={0}>
            <code>{converted.command}</code>
          </pre>
        </div>
      </div>

      <p className="convert__equivalence" data-equivalence={conversion.equivalence} role="note">
        {EQUIVALENCE_NOTES[conversion.equivalence]}
      </p>

      {conversion.notes && <p className="explain__note">{conversion.notes}</p>}

      <ConversionNotes label="What behaves differently" items={conversion.differences} />
      <ConversionNotes label="What did not carry over" items={conversion.unsupported} />

      <div className="convert__safety">
        <RiskBadge risk={converted.riskLevel} reasons={converted.localReasons} />
        {(converted.localReasons.length > 0 || converted.aiReasons.length > 0) && (
          <ul className="explain__reasons">
            {converted.localReasons.map((reason) => (
              <li key={`local-${reason}`}>
                <span className="explain__source">Command Center</span> {reason}
              </li>
            ))}
            {converted.aiReasons.map((reason) => (
              <li key={`ai-${reason}`}>
                <span className="explain__source explain__source--ai">OpenAI</span> {reason}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="convert__actions">
        <Button variant="ghost" size="sm" onClick={() => onCopy(converted.command)}>
          <Copy size={14} aria-hidden="true" />
          Copy
        </Button>
        <Button variant="secondary" size="sm" onClick={() => onReview(converted)}>
          <FilePlus2 size={14} aria-hidden="true" />
          Review and save
        </Button>
      </div>

      <p className="explain__origin">
        Converted by AI. The original entry is untouched, and saving this creates a new one.
      </p>
    </div>
  );
}

function ConversionNotes({ label, items }: { label: string; items: string[] }) {
  if (items.length === 0) return null;

  return (
    <div className="explain__group">
      <p className="card__section-label">{label}</p>
      <ul className="explain__notes">
        {items.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </div>
  );
}
