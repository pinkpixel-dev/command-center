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
      </div>
    </Modal>
  );
}
