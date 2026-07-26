import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { FileText, FolderOpen, ScanLine } from "lucide-react";

import { Button } from "../ui/Button";

export interface ImportSourceProps {
  busy: boolean;
  onScanFile: (path: string) => void;
  onScanText: (content: string) => void;
}

const FILE_FILTERS = [
  { name: "Documents", extensions: ["md", "markdown", "mdx", "txt", "text", "rst", "adoc", "org"] },
];

/** Step one: get a document in. Drop it, pick it, or paste it. */
export function ImportSource({ busy, onScanFile, onScanText }: ImportSourceProps) {
  const [text, setText] = useState("");
  const [hovering, setHovering] = useState(false);
  const [dropError, setDropError] = useState<string | null>(null);

  // Held in a ref so the subscription below can run exactly once, no matter how
  // often the parent re-renders with a fresh callback.
  const scanFileRef = useRef(onScanFile);
  scanFileRef.current = onScanFile;

  // Tauri reports drops at the window level, so this listener lives with the
  // only screen that wants them.
  useEffect(() => {
    const pending = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setHovering(true);
        return;
      }
      if (event.payload.type === "drop") {
        setHovering(false);
        const [first] = event.payload.paths;
        if (first) {
          setDropError(null);
          scanFileRef.current(first);
        } else {
          setDropError("That drop did not contain a file");
        }
        return;
      }
      setHovering(false);
    });

    return () => {
      void pending.then((stop) => stop());
    };
  }, []);

  const pickFile = async () => {
    const selected = await open({ multiple: false, directory: false, filters: FILE_FILTERS });
    if (typeof selected === "string") {
      onScanFile(selected);
    }
  };

  return (
    <div className="import-source">
      <div className={`dropzone${hovering ? " is-hovering" : ""}`} aria-live="polite">
        <FileText size={22} aria-hidden="true" className="dropzone__icon" />
        <p className="dropzone__title">
          {hovering ? "Drop it anywhere" : "Drop a Markdown or text file here"}
        </p>
        <p className="dropzone__body">
          README files, cheat sheets, documentation excerpts, or your own notes. Nothing is saved
          until you review what was found.
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
            onClick={() => onScanText(text)}
            disabled={text.trim().length === 0}
            loading={busy}
          >
            <ScanLine size={15} aria-hidden="true" />
            Scan for commands
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
