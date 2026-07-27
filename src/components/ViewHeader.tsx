import type { ReactNode } from "react";
import { Menu } from "lucide-react";

import { Button } from "./ui/Button";

export interface ViewHeaderProps {
  title: string;
  subtitle?: string;
  onOpenMenu: () => void;
  actions?: ReactNode;
}

/** Top bar for the screens that are not the command list. */
export function ViewHeader({ title, subtitle, onOpenMenu, actions }: ViewHeaderProps) {
  return (
    <header className="topbar">
      <div className="topbar__row">
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          className="topbar__menu"
          aria-label="Open navigation"
          onClick={onOpenMenu}
        >
          <Menu size={18} aria-hidden="true" />
        </Button>
        <div className="topbar__heading">
          <h1>{title}</h1>
          {subtitle && <p>{subtitle}</p>}
        </div>
        {actions && <div className="topbar__actions">{actions}</div>}
      </div>
    </header>
  );
}
