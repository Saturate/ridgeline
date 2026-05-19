import { Clock, ExternalLink, FileEdit, GitBranch } from "lucide-react";
import { parseConventionalCommit, getTypeColor } from "@/lib/conventional-commit";
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
import type { PullRequest, ProviderIndicator } from "@/lib/types";
import type { PrListVariant } from "./pr-list";

interface PrRowProps {
  pr: PullRequest;
  variant: PrListVariant;
  providerColor?: string;
  indicatorMode: ProviderIndicator;
  warningHours?: number;
  dangerHours?: number;
  showProjectName?: boolean;
  parseCC?: boolean;
  onClick: () => void;
}

export function PrRow({
  pr,
  variant,
  providerColor,
  indicatorMode,
  warningHours,
  dangerHours,
  showProjectName = true,
  parseCC = false,
  onClick,
}: PrRowProps) {
  const cc = parseCC ? parseConventionalCommit(pr.title) : null;
  return (
    <div
      className="relative flex cursor-pointer items-center gap-3 px-4 py-3 transition-colors hover:bg-accent/50"
      onClick={onClick}
      style={
        indicatorMode === "border" && providerColor
          ? { borderLeft: `3px solid ${providerColor}` }
          : undefined
      }
    >
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          {indicatorMode === "badge" && providerColor && (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span
                    className="inline-flex shrink-0 items-center rounded px-1.5 py-0.5 text-[10px] font-medium text-white"
                    style={{ backgroundColor: providerColor }}
                  >
                    {pr.id.provider}
                  </span>
                </TooltipTrigger>
                <TooltipContent>{pr.id.provider}</TooltipContent>
              </Tooltip>
            </TooltipProvider>
          )}
          <span className="truncate text-sm font-medium">
            {cc ? cc.description : pr.title}
          </span>
          {pr.isDraft && (
            <Badge variant="outline" className="shrink-0 text-xs">
              <FileEdit className="mr-1 h-3 w-3" />
              Draft
            </Badge>
          )}
        </div>

        {cc && (
          <div className="mt-0.5 flex items-center gap-1">
            <span
              className={`inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium ${getTypeColor(cc.type)}`}
            >
              {cc.type}
            </span>
            {cc.scope && (
              <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                {cc.scope}
              </span>
            )}
            {cc.breaking && (
              <span className="rounded bg-red-500/20 px-1.5 py-0.5 text-[10px] font-medium text-red-600 dark:text-red-400">
                breaking
              </span>
            )}
          </div>
        )}

        <div className="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground">
          <span className="font-medium text-foreground/70">
            {showProjectName
              ? `${pr.repository.project}/${pr.repository.name}`
              : pr.repository.name}
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
        <BuildStatusBadge status={pr.buildStatus} />
        <AgeIndicator createdAt={pr.createdAt} variant={variant} warningHours={warningHours} dangerHours={dangerHours} />
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
