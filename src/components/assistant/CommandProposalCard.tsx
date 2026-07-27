import { Copy, FilePlus2 } from "lucide-react";

import type { CommandProposal } from "../../lib/types";
import { Button } from "../ui/Button";
import { RiskBadge } from "../ui/RiskBadge";

export interface CommandProposalCardProps {
  proposal: CommandProposal;
  onCopy: (text: string) => void;
  onReview: (proposal: CommandProposal) => void;
}

/**
 * A suggested command. It is never run and never saved from here: Copy puts it
 * on the clipboard, and Review and save opens the normal entry form so nothing
 * reaches the library without the user filling it in.
 */
export function CommandProposalCard({ proposal, onCopy, onReview }: CommandProposalCardProps) {
  return (
    <article className="proposal" data-risk={proposal.riskLevel}>
      <header className="proposal__head">
        <h4 className="proposal__title">{proposal.title}</h4>
        <RiskBadge risk={proposal.riskLevel} reasons={proposal.localReasons} />
      </header>

      <pre className="code proposal__code" tabIndex={0}>
        <code>{proposal.command}</code>
      </pre>

      {proposal.why && <p className="proposal__why">{proposal.why}</p>}

      {(proposal.localReasons.length > 0 || proposal.aiReasons.length > 0) && (
        <ul className="proposal__reasons">
          {proposal.localReasons.map((reason) => (
            <li key={`local-${reason}`}>
              <span className="explain__source">Command Center</span> {reason}
            </li>
          ))}
          {proposal.aiReasons.map((reason) => (
            <li key={`ai-${reason}`}>
              <span className="explain__source explain__source--ai">OpenAI</span> {reason}
            </li>
          ))}
        </ul>
      )}

      <div className="proposal__actions">
        <Button variant="ghost" size="sm" onClick={() => onCopy(proposal.command)}>
          <Copy size={14} aria-hidden="true" />
          Copy
        </Button>
        <Button variant="secondary" size="sm" onClick={() => onReview(proposal)}>
          <FilePlus2 size={14} aria-hidden="true" />
          Review and save
        </Button>
      </div>
    </article>
  );
}
