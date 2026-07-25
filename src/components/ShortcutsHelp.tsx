import { prettyShortcut } from "../lib/format";
import { Button } from "./ui/Button";
import { Kbd } from "./ui/Kbd";
import { Modal } from "./ui/Modal";

const SHORTCUTS: { keys: string; action: string }[] = [
  { keys: "Ctrl + K", action: "Focus search" },
  { keys: "/", action: "Focus search" },
  { keys: "N", action: "Add a command" },
  { keys: "↑ ↓", action: "Move through the list" },
  { keys: "Enter", action: "Expand or collapse an entry" },
  { keys: "Esc", action: "Close a dialog or clear the search" },
  { keys: "?", action: "Show this list" },
];

export function ShortcutsHelp({
  open,
  quickAddShortcut,
  onClose,
}: {
  open: boolean;
  quickAddShortcut: string;
  onClose: () => void;
}) {
  return (
    <Modal
      open={open}
      title="Keyboard shortcuts"
      onClose={onClose}
      footer={
        <Button variant="secondary" onClick={onClose}>
          Close
        </Button>
      }
    >
      <ul className="shortcut-list">
        {SHORTCUTS.map((shortcut) => (
          <li key={shortcut.action + shortcut.keys}>
            <span>{shortcut.action}</span>
            <Kbd keys={shortcut.keys} />
          </li>
        ))}
        <li>
          <span>Open Quick Add from anywhere</span>
          <Kbd keys={prettyShortcut(quickAddShortcut)} />
        </li>
      </ul>
    </Modal>
  );
}
