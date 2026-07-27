/**
 * One counter for every cancellable AI request in the window.
 *
 * Rust tracks in-flight requests in a single registry keyed by this id, and
 * registering a duplicate aborts whatever was already running under it. The
 * assistant, error analysis, and shell conversion can all have a request on the
 * wire at once, so they draw from here rather than each counting from one.
 */
let issued = 0;

export function nextRequestId(): number {
  issued += 1;
  return issued;
}
