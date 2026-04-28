type ExplainableViolation = {
  operation: string;
  target: string;
  ruleId?: string;
  message?: string;
};

export function explainViolation(event: ExplainableViolation): string {
  return `Blocked ${event.operation} on ${event.target}. Rule ${event.ruleId} fired. ${event.message}`;
}
