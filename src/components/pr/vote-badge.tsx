import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { Reviewer, Vote } from "@/lib/types";

interface VoteBadgeProps {
  reviewer: Reviewer;
}

const voteConfig: Record<Vote, { symbol: string; color: string }> = {
  Approved: { symbol: "✓", color: "bg-green-500/20 text-green-700 dark:text-green-400" },
  ApprovedWithSuggestions: { symbol: "✓~", color: "bg-green-500/10 text-green-600 dark:text-green-500" },
  NoVote: { symbol: "·", color: "bg-muted text-muted-foreground" },
  WaitingForAuthor: { symbol: "⏳", color: "bg-yellow-500/20 text-yellow-700 dark:text-yellow-400" },
  Rejected: { symbol: "✗", color: "bg-red-500/20 text-red-700 dark:text-red-400" },
};

function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .map((w) => w[0])
    .slice(0, 2)
    .join("")
    .toUpperCase();
}

export function VoteBadge({ reviewer }: VoteBadgeProps) {
  const config = voteConfig[reviewer.vote];
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            className={cn(
              "inline-flex h-6 items-center gap-0.5 rounded-full px-1.5 text-xs font-medium",
              config.color,
              reviewer.isRequired && "ring-1 ring-current/30",
            )}
          >
            <span>{config.symbol}</span>
            <span>{initials(reviewer.user.displayName)}</span>
          </span>
        </TooltipTrigger>
        <TooltipContent>
          {reviewer.user.displayName}
          {reviewer.isRequired ? " (required)" : ""}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
