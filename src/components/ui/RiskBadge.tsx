import { AlertOctagon, AlertTriangle, ShieldCheck } from "lucide-react";

import { riskLabel } from "../../lib/format";
import type { RiskLevel } from "../../lib/types";

const ICONS = {
  safe: ShieldCheck,
  caution: AlertTriangle,
  destructive: AlertOctagon,
} as const;

interface RiskBadgeProps {
  risk: RiskLevel;
  reasons?: string[];
  iconOnly?: boolean;
}

/** The compact form keeps distinct shapes and an accessible label, not colour alone. */
export function RiskBadge({ risk, reasons = [], iconOnly = false }: RiskBadgeProps) {
  const Icon = ICONS[risk];
  const label = riskLabel(risk);
  const tooltip = reasons.length > 0 ? `${label}: ${reasons.join(". ")}` : `Risk: ${label}`;

  return (
    <span
      className={`risk risk--${risk}${iconOnly ? " risk--icon" : ""}`}
      title={tooltip}
      aria-label={iconOnly ? tooltip : undefined}
    >
      <Icon size={iconOnly ? 17 : 13} aria-hidden="true" />
      {!iconOnly && label}
    </span>
  );
}
