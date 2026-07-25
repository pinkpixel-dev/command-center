/** Renders a key hint like `Ctrl + K` as separate key caps. */
export function Kbd({ keys }: { keys: string }) {
  const parts = keys.split("+").map((part) => part.trim());

  return (
    <span className="kbd-group">
      {parts.map((part, index) => (
        <kbd key={`${part}-${index}`} className="kbd">
          {part}
        </kbd>
      ))}
    </span>
  );
}
