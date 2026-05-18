import { AlertTriangle, CheckCircle, Clock, UserX } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { PullRequest } from "@/lib/types";

interface PrStatusSummaryProps {
  pr: PullRequest;
}

export function PrStatusSummary({ pr }: PrStatusSummaryProps) {
  if (pr.reviewers.length === 0) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Badge
              variant="outline"
              className="gap-1 border-orange-500/50 text-orange-600 dark:text-orange-400"
            >
              <UserX className="h-3 w-3" />
              No reviewers
            </Badge>
          </TooltipTrigger>
          <TooltipContent>
            No reviewers assigned — this PR won't get votes
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  const approved = pr.reviewers.filter(
    (r) =>
      r.vote === "Approved" || r.vote === "ApprovedWithSuggestions",
  ).length;
  const rejected = pr.reviewers.filter(
    (r) => r.vote === "Rejected",
  ).length;
  const waitingForAuthor = pr.reviewers.filter(
    (r) => r.vote === "WaitingForAuthor",
  ).length;
  const total = pr.reviewers.length;

  if (rejected > 0) {
    return (
      <Badge
        variant="destructive"
        className="gap-1"
      >
        <AlertTriangle className="h-3 w-3" />
        {rejected} rejected
      </Badge>
    );
  }

  if (waitingForAuthor > 0) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Badge
              variant="outline"
              className="gap-1 border-yellow-500/50 text-yellow-600 dark:text-yellow-400"
            >
              <Clock className="h-3 w-3" />
              Waiting for you
            </Badge>
          </TooltipTrigger>
          <TooltipContent>
            {waitingForAuthor} reviewer{waitingForAuthor > 1 ? "s" : ""} requested changes
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  if (approved === total) {
    return (
      <Badge className="gap-1 bg-green-600 hover:bg-green-600">
        <CheckCircle className="h-3 w-3" />
        All approved
      </Badge>
    );
  }

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Badge variant="secondary" className="gap-1">
            <CheckCircle className="h-3 w-3" />
            {approved}/{total}
          </Badge>
        </TooltipTrigger>
        <TooltipContent>
          {approved} of {total} reviewers approved
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
