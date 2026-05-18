import { AlertTriangle, GitMerge } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { MergeStatus, BuildStatus } from "@/lib/types";

interface MergeStatusBadgeProps {
  status: MergeStatus | null;
}

export function MergeStatusBadge({ status }: MergeStatusBadgeProps) {
  if (!status || status === "Succeeded" || status === "NotSet" || status === "Queued") {
    return null;
  }

  const config = status === "Conflicts"
    ? { label: "Conflicts", icon: GitMerge }
    : { label: "Policy blocked", icon: AlertTriangle };

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Badge variant="destructive" className="gap-1 text-xs">
            <config.icon className="h-3 w-3" />
            {config.label}
          </Badge>
        </TooltipTrigger>
        <TooltipContent>
          {status === "Conflicts"
            ? "This PR has merge conflicts"
            : "Blocked by branch policy"}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

interface BuildStatusBadgeProps {
  status: BuildStatus | null;
}

export function BuildStatusBadge({ status }: BuildStatusBadgeProps) {
  if (!status || status === "Succeeded" || status === "NotStarted") {
    return null;
  }

  if (status === "InProgress") {
    return (
      <Badge variant="secondary" className="gap-1 text-xs">
        <span className="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
        Building
      </Badge>
    );
  }

  return (
    <Badge variant="destructive" className="gap-1 text-xs">
      Build failed
    </Badge>
  );
}
