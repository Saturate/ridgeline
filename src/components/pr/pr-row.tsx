import { Clock, ExternalLink, FileEdit, GitBranch } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { VoteBadge } from "./vote-badge";
import { AgeIndicator } from "./age-indicator";
import { MergeStatusBadge, BuildStatusBadge } from "./merge-status-badge";
import { PrStatusSummary } from "./pr-status-summary";
import { api } from "@/lib/api";
import type { PullRequest } from "@/lib/types";
import type { PrListVariant } from "./pr-list";

interface PrRowProps {
  pr: PullRequest;
  variant: PrListVariant;
  onClick: () => void;
}

export function PrRow({ pr, variant, onClick }: PrRowProps) {
  return (
    <div
      className="flex cursor-pointer items-center gap-3 px-4 py-3 transition-colors hover:bg-accent/50"
      onClick={onClick}
    >
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium">{pr.title}</span>
          {pr.isDraft && (
            <Badge variant="outline" className="shrink-0 text-xs">
              <FileEdit className="mr-1 h-3 w-3" />
              Draft
            </Badge>
          )}
        </div>

        <div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
          <span className="font-medium text-foreground/70">
            {pr.repository.project}/{pr.repository.name}
          </span>
          <span>#{pr.id.number}</span>
          {variant === "reviewing" && (
            <span>by {pr.author.displayName}</span>
          )}
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="inline-flex items-center gap-1">
                  <GitBranch className="h-3 w-3" />
                  {pr.sourceBranch}
                </span>
              </TooltipTrigger>
              <TooltipContent>
                {pr.sourceBranch} → {pr.targetBranch}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-2">
        {variant === "authored" ? (
          <PrStatusSummary pr={pr} />
        ) : (
          <>
            {pr.reviewers.some((r) => r.vote === "WaitingForAuthor") && (
              <Badge
                variant="outline"
                className="gap-1 border-yellow-500/50 text-xs text-yellow-600 dark:text-yellow-400"
              >
                <Clock className="h-3 w-3" />
                Waiting for author
              </Badge>
            )}
            <div className="flex items-center gap-0.5">
              {pr.reviewers.map((r) => (
                <VoteBadge key={r.user.id} reviewer={r} />
              ))}
            </div>
          </>
        )}
        <MergeStatusBadge status={pr.mergeStatus} />
        <BuildStatusBadge status={null} />
        <AgeIndicator createdAt={pr.createdAt} />
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={(e: React.MouseEvent) => {
            e.stopPropagation();
            api.openUrl(pr.webUrl);
          }}
        >
          <ExternalLink className="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>
  );
}
