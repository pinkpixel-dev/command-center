import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

export interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  body: string;
  action?: ReactNode;
}

export function EmptyState({ icon: Icon, title, body, action }: EmptyStateProps) {
  return (
    <div className="empty-state">
      <Icon size={22} aria-hidden="true" className="empty-state__icon" />
      <h3>{title}</h3>
      <p>{body}</p>
      {action && <div className="empty-state__action">{action}</div>}
    </div>
  );
}
