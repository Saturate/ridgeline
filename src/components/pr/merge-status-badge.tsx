import { Badge } from "@/components/ui/badge";
import type { MergeStatus } from "@/lib/types";

interface MergeStatusBadgeProps {
  status: MergeStatus;
}

const statusConfig: Record<MergeStatus, { label: string; variant: "default" | "destructive" | "secondary" | "outline" }> = {
  Succeeded: { label: "Ready", variant: "default" },
  Conflicts: { label: "Conflicts", variant: "destructive" },
  RejectedByPolicy: { label: "Policy", variant: "destructive" },
  NotSet: { label: "Pending", variant: "secondary" },
  Queued: { label: "Queued", variant: "secondary" },
};

export function MergeStatusBadge({ status }: MergeStatusBadgeProps) {
  const config = statusConfig[status];
  return (
    <Badge variant={config.variant} className="text-xs">
      {config.label}
    </Badge>
  );
}
