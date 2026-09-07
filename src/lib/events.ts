/**
 * The names the backend emits notifications under. Both platform
 * implementations and every listening hook use these, so they live somewhere
 * neither side has to import the other to reach.
 */
export const LIBRARY_CHANGED = "library-changed";
export const SETTINGS_CHANGED = "settings-changed";
