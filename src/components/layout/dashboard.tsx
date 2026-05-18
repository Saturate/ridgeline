import { useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { AlertCircle, Search } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Skeleton } from "@/components/ui/skeleton";
import { usePollData } from "@/lib/hooks/use-poll-data";
import { PrList } from "@/components/pr/pr-list";
import { PrDetailPanel } from "@/components/pr/pr-detail-panel";
import type { PrId, PullRequest } from "@/lib/types";

interface DashboardProps {
  initialized: boolean;
  initError: string | null;
}

export function Dashboard({ initialized, initError }: DashboardProps) {
  const [search, setSearch] = useState("");
  const [selectedPrId, setSelectedPrId] = useState<PrId | null>(null);
  const { data, isLoading, error } = usePollData(initialized);

  if (initError) {
    return (
      <div className="flex items-center justify-center p-8">
        <Alert variant="destructive" className="max-w-lg">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{initError}</AlertDescription>
        </Alert>
      </div>
    );
  }

  if (!initialized || isLoading) {
    return (
      <div className="space-y-3 p-4">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-16 w-full" />
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center p-8">
        <Alert variant="destructive" className="max-w-lg">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{String(error)}</AlertDescription>
        </Alert>
      </div>
    );
  }

  const reviewing = filterPrs(data?.reviewing ?? [], search);
  const authored = filterPrs(data?.authored ?? [], search);

  return (
    <div className="flex h-full flex-col">
      {data?.errors && data.errors.length > 0 && (
        <div className="border-b px-4 py-2">
          {data.errors.map((err, i) => (
            <Alert key={i} variant="destructive" className="mb-1">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                {err.provider}: {err.message}
              </AlertDescription>
            </Alert>
          ))}
        </div>
      )}

      <div className="border-b px-4 py-2">
        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search PRs..."
            value={search}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSearch(e.target.value)}
            className="h-9 pl-9"
          />
        </div>
      </div>

      <Tabs defaultValue="reviewing" className="flex flex-1 flex-col">
        <div className="border-b px-4">
          <TabsList className="h-9">
            <TabsTrigger value="reviewing" className="gap-1.5 text-xs">
              Reviewing
              <Badge variant="secondary" className="h-5 px-1.5 text-xs">
                {reviewing.length}
              </Badge>
            </TabsTrigger>
            <TabsTrigger value="authored" className="gap-1.5 text-xs">
              Authored
              <Badge variant="secondary" className="h-5 px-1.5 text-xs">
                {authored.length}
              </Badge>
            </TabsTrigger>
          </TabsList>
        </div>

        <ScrollArea className="flex-1">
          <TabsContent value="reviewing" className="m-0">
            <PrList prs={reviewing} onSelect={setSelectedPrId} />
          </TabsContent>
          <TabsContent value="authored" className="m-0">
            <PrList prs={authored} onSelect={setSelectedPrId} />
          </TabsContent>
        </ScrollArea>
      </Tabs>

      <PrDetailPanel
        prId={selectedPrId}
        onClose={() => setSelectedPrId(null)}
      />
    </div>
  );
}

function filterPrs(prs: PullRequest[], search: string): PullRequest[] {
  if (!search.trim()) return prs;
  const q = search.toLowerCase();
  return prs.filter(
    (pr) =>
      pr.title.toLowerCase().includes(q) ||
      pr.author.displayName.toLowerCase().includes(q) ||
      pr.repository.name.toLowerCase().includes(q) ||
      pr.repository.project.toLowerCase().includes(q) ||
      pr.sourceBranch.toLowerCase().includes(q) ||
      String(pr.id.number).includes(q),
  );
}
