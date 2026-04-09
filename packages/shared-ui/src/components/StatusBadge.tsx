import type { PropsWithChildren } from "react";

export type StatusBadgeTone = "info" | "warn" | "error";

type StatusBadgeProps = PropsWithChildren<{
  tone?: StatusBadgeTone;
}>;

export function StatusBadge({ tone = "info", children }: StatusBadgeProps) {
  return (
    <span className={`rampart-badge rampart-badge--${tone}`}>{children}</span>
  );
}

