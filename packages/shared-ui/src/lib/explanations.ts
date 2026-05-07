type ViolationExplanation = {
  ruleDescription: string;
  platformLimitation: { detail: string } | null;
  remediationHint: string | null;
};

type ExplainableViolation = {
  operation: string;
  target: string;
  ruleId?: string;
  ruleLabel?: string;
  platformNote?: string | null;
  message?: string;
  explanation?: ViolationExplanation | null;
};

export function explainViolation(event: ExplainableViolation): string {
  if (event.explanation) {
    const parts = [event.explanation.ruleDescription.trim()];
    if (event.explanation.remediationHint) {
      parts.push(event.explanation.remediationHint.trim());
    }
    return parts.join(" ");
  }
  const ruleClause = event.ruleId ? ` Rule ${event.ruleId} fired.` : "";
  const messageClause = event.message ? ` ${event.message}` : "";
  return `Blocked ${event.operation} on ${event.target}.${ruleClause}${messageClause}`.trim();
}
