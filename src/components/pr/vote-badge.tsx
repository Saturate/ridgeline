import { Check, Clock, Minus, Users, X } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { Reviewer, Vote } from "@/lib/types";
import type { LucideIcon } from "lucide-react";

interface VoteBadgeProps {
  reviewer: Reviewer;
}

const voteConfig: Record<Vote, { icon: LucideIcon; color: string }> = {
  Approved: { icon: Check, color: "bg-green-500/20 text-green-700 dark:text-green-400" },
  ApprovedWithSuggestions: { icon: Check, color: "bg-green-500/10 text-green-600 dark:text-green-500" },
  NoVote: { icon: Minus, color: "bg-muted text-muted-foreground" },
  WaitingForAuthor: { icon: Clock, color: "bg-yellow-500/20 text-yellow-700 dark:text-yellow-400" },
  Rejected: { icon: X, color: "bg-red-500/20 text-red-700 dark:text-red-400" },
};

function isGroup(reviewer: Reviewer): boolean {
  return /^\[.*?\\/.test(reviewer.user.displayName);
}

function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .map((w) => w[0])
    .slice(0, 2)
    .join("")
    .toUpperCase();
}

function groupLabel(name: string): string {
  return name.replace(/^\[.*?\\/, "");
}

export function VoteBadge({ reviewer }: VoteBadgeProps) {
  const config = voteConfig[reviewer.vote];
  const Icon = config.icon;
  const group = isGroup(reviewer);
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
            <Icon className="h-3 w-3" />
            {group ? (
              <Users className="h-3 w-3" />
            ) : (
              <span>{initials(reviewer.user.displayName)}</span>
            )}
          </span>
        </TooltipTrigger>
        <TooltipContent>
          {group ? groupLabel(reviewer.user.displayName) : reviewer.user.displayName}
          {reviewer.isRequired ? " (required)" : ""}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
