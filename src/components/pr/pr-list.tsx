import { Inbox } from "lucide-react";
import { PrRow } from "./pr-row";
import type { PrId, PullRequest, ProviderIndicator } from "@/lib/types";

export type PrListVariant = "reviewing" | "authored";

interface PrListProps {
  prs: PullRequest[];
  variant: PrListVariant;
  providerColors: Record<string, string>;
  indicatorMode: ProviderIndicator;
  onSelect: (prId: PrId) => void;
}

export function PrList({
  prs,
  variant,
  providerColors,
  indicatorMode,
  onSelect,
}: PrListProps) {
  if (prs.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center gap-2 py-16 text-muted-foreground">
        <Inbox className="h-10 w-10" />
        <p className="text-sm">No pull requests</p>
      </div>
    );
  }

  return (
    <div className="divide-y">
      {prs.map((pr) => (
        <PrRow
          key={prKey(pr)}
          pr={pr}
          variant={variant}
          providerColor={providerColors[pr.id.provider]}
          indicatorMode={indicatorMode}
          onClick={() => onSelect(pr.id)}
        />
      ))}
    </div>
  );
}

function prKey(pr: PullRequest): string {
  return `${pr.id.provider}:${pr.id.project}:${pr.id.repository}:${pr.id.number}`;
}
