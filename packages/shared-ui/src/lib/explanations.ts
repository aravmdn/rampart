import type { ViolationEvent } from "../../../../apps/desktop/src/daemon/contracts";

export function explainViolation(event: ViolationEvent): string {
  return `Blocked ${event.operation} on ${event.target}. Rule ${event.ruleId} fired. ${event.message}`;
}
