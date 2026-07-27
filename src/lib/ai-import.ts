/**
 * Import-specific pieces of the AI-assisted import flow. The outbound
 * disclosure itself is shared with terminal error analysis and lives in
 * `outbound.ts`; it is re-exported here so import callers keep one import path.
 */

export type { OutboundPlan as AiImportPlan, PlannedRedaction } from "./outbound";
export {
  describeOutbound,
  describeRedactions,
  formatBytes,
  groupRedactions,
  redactionLabel,
} from "./outbound";

/** A document read by Rust so the webview never touches the filesystem. */
export interface ImportDocument {
  name: string | null;
  content: string;
}
