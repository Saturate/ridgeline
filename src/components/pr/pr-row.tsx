import { ExternalLink, FileEdit, GitBranch } from "lucide-react";
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
import { MergeStatusBadge } from "./merge-status-badge";
import { api } from "@/lib/api";
import type { PullRequest } from "@/lib/types";

interface PrRowProps {
  pr: PullRequest;
  onClick: () => void;
}

export function PrRow({ pr, onClick }: PrRowProps) {
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
          <span>by {pr.author.displayName}</span>
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
        <div className="flex items-center gap-0.5">
          {pr.reviewers.map((r) => (
            <VoteBadge key={r.user.id} reviewer={r} />
          ))}
        </div>
        {pr.mergeStatus && <MergeStatusBadge status={pr.mergeStatus} />}
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
