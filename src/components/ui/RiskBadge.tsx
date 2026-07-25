import { AlertOctagon, AlertTriangle, ShieldCheck } from "lucide-react";

import { riskLabel } from "../../lib/format";
import type { RiskLevel } from "../../lib/types";

const ICONS = {
  safe: ShieldCheck,
  caution: AlertTriangle,
  destructive: AlertOctagon,
} as const;

/** Icon + word, never colour on its own. */
export function RiskBadge({ risk, reasons = [] }: { risk: RiskLevel; reasons?: string[] }) {
  const Icon = ICONS[risk];
  const label = riskLabel(risk);

  return (
    <span
      className={`risk risk--${risk}`}
      title={reasons.length > 0 ? reasons.join(". ") : `Risk: ${label}`}
    >
      <Icon size={13} aria-hidden="true" />
      {label}
    </span>
  );
}
