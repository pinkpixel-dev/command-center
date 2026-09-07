import { useEffect, useRef, useState } from "react";
import { FileText, FolderOpen, ScanLine } from "lucide-react";

import { platform } from "../../lib/platform";
import type { ImportDocumentReader } from "../../lib/platform";
import { Button } from "../ui/Button";

export interface ImportSourceProps {
  busy: boolean;
  /** Restores the pasted text when the user backs out of the disclosure step. */
  initialText?: string;
  onReadFile: (read: ImportDocumentReader) => void;
  onReadText: (content: string) => void;
}

/** Step one: get a document in. Drop it, pick it, or paste it. */
export function ImportSource({
  busy,
  initialText = "",
  onReadFile,
  onReadText,
}: ImportSourceProps) {
  const [text, setText] = useState(initialText);
  const [hovering, setHovering] = useState(false);
  const [dropError, setDropError] = useState<string | null>(null);

  // Held in a ref so the subscription below can run exactly once, no matter how
  // often the parent re-renders with a fresh callback.
  const readFileRef = useRef(onReadFile);
  readFileRef.current = onReadFile;

  // Drops are watched at the window level on both platforms, so this
  // subscription lives with the only screen that wants them.
  useEffect(
    () =>
      platform.watchFileDrops({
        onOver: () => setHovering(true),
        onLeave: () => setHovering(false),
        onDrop: (read) => {
          setHovering(false);
          if (read) {
            setDropError(null);
            readFileRef.current(read);
          } else {
            setDropError("That drop did not contain a file");
          }
        },
      }),
    [],
  );

  const pickFile = async () => {
    const read = await platform.chooseImportDocument();
    if (read) onReadFile(read);
  };

  return (
    <div className="import-source">
      <div className={`dropzone${hovering ? " is-hovering" : ""}`} aria-live="polite">
        <FileText size={22} aria-hidden="true" className="dropzone__icon" />
        <p className="dropzone__title">
          {hovering ? "Drop it anywhere" : "Drop a Markdown or text file here"}
        </p>
        <p className="dropzone__body">
          README files, cheat sheets, documentation excerpts, or your own notes. You see exactly
          what would be sent to OpenAI before it leaves this machine, and nothing is saved until
          you review what comes back.
        </p>
        <Button variant="secondary" onClick={() => void pickFile()} disabled={busy}>
          <FolderOpen size={15} aria-hidden="true" />
          Choose a file
        </Button>
        {dropError && (
          <p className="field__error" role="alert">
            <span aria-hidden="true">!</span> {dropError}
          </p>
        )}
      </div>

      <div className="import-source__paste">
        <label className="field__label" htmlFor="import-paste">
          Or paste content
        </label>
        <textarea
          id="import-paste"
          className="input input--textarea input--mono"
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder={"## Docker cleanup\n\n```bash\ndocker container prune\n```"}
          rows={10}
          spellCheck={false}
        />
        <div className="import-source__actions">
          <Button
            variant="primary"
            onClick={() => onReadText(text)}
            disabled={text.trim().length === 0}
            loading={busy}
          >
            <ScanLine size={15} aria-hidden="true" />
            Check what would be sent
          </Button>
          {text.trim().length > 0 && (
            <Button variant="ghost" onClick={() => setText("")} disabled={busy}>
              Clear
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
