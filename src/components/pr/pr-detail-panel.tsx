import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import {
  ExternalLink,
  GitBranch,
  Shield,
  Plus,
  Minus,
  FileEdit,
} from "lucide-react";
import Markdown from "react-markdown";
import { usePrDetail } from "@/lib/hooks/use-pr-detail";
import { VoteBadge } from "./vote-badge";
import { MergeStatusBadge, BuildStatusBadge } from "./merge-status-badge";
import { AgeIndicator } from "./age-indicator";
import { api } from "@/lib/api";
import type { PrId, PolicyStatus } from "@/lib/types";

interface PrDetailPanelProps {
  prId: PrId | null;
  onClose: () => void;
}

export function PrDetailPanel({ prId, onClose }: PrDetailPanelProps) {
  const { data, isLoading } = usePrDetail(prId);

  return (
    <Sheet open={prId !== null} onOpenChange={() => onClose()}>
      <SheetContent className="w-[480px] sm:max-w-[480px]">
        {isLoading && (
          <div className="space-y-4 pt-6">
            <Skeleton className="h-6 w-3/4" />
            <Skeleton className="h-4 w-1/2" />
            <Skeleton className="h-32 w-full" />
          </div>
        )}
        {data && (
          <>
            <SheetHeader>
              <div className="flex items-start gap-2">
                <SheetTitle className="text-left text-base leading-tight">
                  {data.pr.title}
                </SheetTitle>
                {data.pr.isDraft && (
                  <Badge variant="outline" className="shrink-0">
                    <FileEdit className="mr-1 h-3 w-3" />
                    Draft
                  </Badge>
                )}
              </div>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span>
                  {data.pr.repository.project}/{data.pr.repository.name}
                </span>
                <span>#{data.pr.id.number}</span>
              </div>
            </SheetHeader>

            <div className="mt-4 space-y-4">
              <div className="flex items-center gap-2 text-sm">
                <span className="text-muted-foreground">By</span>
                <span className="font-medium">
                  {data.pr.author.displayName}
                </span>
                <AgeIndicator createdAt={data.pr.createdAt} variant="authored" />
              </div>

              <div className="flex items-center gap-2 text-sm">
                <GitBranch className="h-4 w-4 text-muted-foreground" />
                <code className="rounded bg-muted px-1.5 py-0.5 text-xs">
                  {data.pr.sourceBranch}
                </code>
                <span className="text-muted-foreground">→</span>
                <code className="rounded bg-muted px-1.5 py-0.5 text-xs">
                  {data.pr.targetBranch}
                </code>
              </div>

              <div className="flex items-center gap-2">
                <MergeStatusBadge status={data.pr.mergeStatus} />
                <BuildStatusBadge status={data.pr.buildStatus} />
              </div>

              {data.diffStats && (
                <div className="flex items-center gap-3 text-sm">
                  <span className="text-muted-foreground">
                    {data.diffStats.filesChanged} files
                  </span>
                  <span className="inline-flex items-center gap-1 text-green-600 dark:text-green-400">
                    <Plus className="h-3 w-3" />
                    {data.diffStats.additions}
                  </span>
                  <span className="inline-flex items-center gap-1 text-red-600 dark:text-red-400">
                    <Minus className="h-3 w-3" />
                    {data.diffStats.deletions}
                  </span>
                </div>
              )}

              <Separator />

              <div>
                <h3 className="mb-2 text-sm font-medium">Reviewers</h3>
                <div className="space-y-2">
                  {data.pr.reviewers.map((r) => (
                    <div
                      key={r.user.id}
                      className="flex items-center justify-between"
                    >
                      <div className="flex items-center gap-2">
                        <VoteBadge reviewer={r} />
                        <span className="text-sm">
                          {r.user.displayName.replace(/^\[.*?\\/, "")}
                        </span>
                      </div>
                      {r.isRequired && (
                        <Badge variant="outline" className="text-xs">
                          Required
                        </Badge>
                      )}
                    </div>
                  ))}
                  {data.pr.reviewers.length === 0 && (
                    <p className="text-sm text-muted-foreground">
                      No reviewers assigned
                    </p>
                  )}
                </div>
              </div>

              {data.policies.length > 0 && (
                <>
                  <Separator />
                  <div>
                    <h3 className="mb-2 text-sm font-medium">Policies</h3>
                    <PolicyList policies={data.policies} />
                  </div>
                </>
              )}

              {data.pr.description && (
                <>
                  <Separator />
                  <div>
                    <h3 className="mb-2 text-sm font-medium">Description</h3>
                    <div className="space-y-2 text-sm text-muted-foreground [&_h1]:font-semibold [&_h1]:text-foreground [&_h2]:font-semibold [&_h2]:text-foreground [&_h3]:font-medium [&_h3]:text-foreground [&_ul]:list-disc [&_ul]:pl-4 [&_ol]:list-decimal [&_ol]:pl-4 [&_li]:my-0.5 [&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_code]:text-xs [&_pre]:rounded-md [&_pre]:bg-muted [&_pre]:p-3 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_a]:text-primary [&_a]:underline [&_strong]:text-foreground [&_blockquote]:border-l-2 [&_blockquote]:border-muted-foreground/30 [&_blockquote]:pl-3 [&_blockquote]:italic">
                      <Markdown>{data.pr.description}</Markdown>
                    </div>
                  </div>
                </>
              )}

              <Separator />

              <Button
                className="w-full"
                onClick={() => api.openUrl(data.pr.webUrl)}
              >
                <ExternalLink className="mr-2 h-4 w-4" />
                Open in Browser
              </Button>
            </div>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}

function PolicyList({ policies }: { policies: PolicyStatus[] }) {
  return (
    <div className="space-y-1.5">
      {policies.map((policy, i) => (
        <div key={i} className="flex items-center justify-between text-sm">
          <div className="flex items-center gap-2">
            <Shield
              className={`h-3.5 w-3.5 ${policyColor(policy.status)}`}
            />
            <span>{policy.name}</span>
          </div>
          <div className="flex items-center gap-1">
            {policy.isBlocking && (
              <Badge variant="outline" className="text-xs">
                Blocking
              </Badge>
            )}
            <Badge
              variant={
                policy.status === "Approved"
                  ? "default"
                  : policy.status === "Rejected"
                    ? "destructive"
                    : "secondary"
              }
              className="text-xs"
            >
              {policy.status}
            </Badge>
          </div>
        </div>
      ))}
    </div>
  );
}

function policyColor(status: string): string {
  switch (status) {
    case "Approved":
      return "text-green-600 dark:text-green-400";
    case "Rejected":
      return "text-red-600 dark:text-red-400";
    case "Running":
      return "text-blue-600 dark:text-blue-400";
    default:
      return "text-muted-foreground";
  }
}
