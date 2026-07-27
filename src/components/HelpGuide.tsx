import { Button } from "./ui/Button";
import { Modal } from "./ui/Modal";

export interface HelpGuideProps {
  open: boolean;
  onClose: () => void;
}

export function HelpGuide({ open, onClose }: HelpGuideProps) {
  return (
    <Modal
      open={open}
      size="lg"
      mobileFullscreen
      title="Help"
      description="The short version of how Command Center works."
      onClose={onClose}
      footer={
        <Button variant="secondary" onClick={onClose}>
          Close
        </Button>
      }
    >
      <div className="help-guide">
        <section>
          <h3>Build your library</h3>
          <p>
            Add a command, script, snippet, or reference. A title and the actual content are the
            only required fields. Tags and collections make larger libraries easier to browse.
          </p>
        </section>

        <section>
          <h3>Find things quickly</h3>
          <p>
            Search checks titles, command text, descriptions, notes, tags, collections, shells,
            and operating systems. Favorites and Recent are useful shortcuts for the entries you
            reach for most.
          </p>
        </section>

        <section>
          <h3>Reuse values with placeholders</h3>
          <p>
            Wrap a name in two pairs of curly braces wherever a value changes. For example:
          </p>
          <pre className="code help-guide__example">
            <code>{"ssh {{user}}@{{host}}"}</code>
          </pre>
          <ol>
            <li>Save the command with the placeholders exactly as shown.</li>
            <li>Open the saved entry and fill the generated user and host fields.</li>
            <li>Check the rendered command, then copy it.</li>
          </ol>
          <p className="help-guide__note">
            Filled values exist only while the entry is open and are never saved to the library.
            The completed command, including any sensitive value, is placed on your system
            clipboard when you copy it.
          </p>
        </section>

        <section>
          <h3>Nothing runs from here</h3>
          <p>
            Command Center copies content but never executes it. Read the rendered command and any
            safety warning before pasting it into a terminal.
          </p>
        </section>

        <section>
          <h3>AI is optional</h3>
          <p>
            AI starts off and the command library works without a key or network connection. Add
            an OpenAI API key and turn AI on in Settings when you want the extra tools. The key is
            stored by your operating system and is never returned to the app interface.
          </p>
          <p className="help-guide__note">
            AI requests are not stored by OpenAI through the API. Command Center checks likely
            secrets locally before sending content, but no detector can promise to find every
            secret. Always read the disclosure before you send a document or terminal output.
          </p>
        </section>

        <section>
          <h3>Import with review</h3>
          <p>
            AI-assisted Import reads a document locally, replaces likely secrets with labelled
            placeholders, and shows exactly what would be sent. You choose whether to continue.
            Extracted entries then pass through the local command, risk, duplicate, and terminal
            output checks before the normal review screen opens. Nothing is saved until you import
            it.
          </p>
        </section>

        <section>
          <h3>Explain a saved entry</h3>
          <p>
            Open an entry and choose Explain for a quick summary or a detailed breakdown of flags,
            pipeline stages, side effects, assumptions, and safety. Explanations are cached on this
            device. Editing the entry marks its explanation stale, and Settings can clear every
            saved explanation even while AI is off.
          </p>
        </section>

        <section>
          <h3>Ask the assistant</h3>
          <p>
            The assistant can suggest a command you do not have or answer follow-up questions
            about a saved entry. Suggested commands are proposals with local risk checks. Copy one
            directly or choose Review and save to open the normal command form. The assistant never
            runs or saves a command on its own, and its conversation is discarded when AI is turned
            off.
          </p>
        </section>

        <section>
          <h3>Analyze terminal errors</h3>
          <p>
            Paste terminal output into the assistant to see the likely cause, the lines that support
            it, a confidence level, and practical checks to try next. The same outbound disclosure
            used by Import appears before anything is sent. A paste is often only part of the
            failure, so the analysis also says what evidence may be missing and keeps the
            line-numbered output available for follow-up questions.
          </p>
        </section>

        <section>
          <h3>Convert between shells</h3>
          <p>
            Open a saved entry to rewrite it for bash, fish, zsh, or PowerShell. Command Center
            shows the original, the proposal, and notes about what changed or could not carry over.
            Every proposal passes through the local risk rules, but shell conversions are never
            guaranteed equivalents. Review the result for quoting, tools, paths, and exit behavior
            before using it.
          </p>
        </section>
      </div>
    </Modal>
  );
}
