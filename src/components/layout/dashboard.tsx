import { useState, useMemo } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { AlertCircle, Search, SlidersHorizontal, FileEdit } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Skeleton } from "@/components/ui/skeleton";
import { usePollData } from "@/lib/hooks/use-poll-data";
import { PrList } from "@/components/pr/pr-list";
import { PrDetailPanel } from "@/components/pr/pr-detail-panel";
import { useConfig } from "@/lib/hooks/use-config";
import { getProviderColorMap } from "@/lib/provider-colors";
import type { PrId, PullRequest } from "@/lib/types";

interface DashboardProps {
  initialized: boolean;
  initError: string | null;
}

export function Dashboard({ initialized, initError }: DashboardProps) {
  const [search, setSearch] = useState("");
  const [selectedPrId, setSelectedPrId] = useState<PrId | null>(null);
  const [hideDrafts, setHideDrafts] = useState(true);
  const [enabledProviders, setEnabledProviders] = useState<Set<string> | null>(
    null,
  );
  const { data, isLoading, error } = usePollData(initialized);
  const { data: config } = useConfig();

  const DEMO_MODE = window.location.search.includes("demo");
  const providerColors = useMemo(
    () =>
      DEMO_MODE
        ? { contoso: "#3b82f6", fabrikam: "#10b981" }
        : getProviderColorMap(config?.providers ?? []),
    [config?.providers, DEMO_MODE],
  );
  const indicatorMode = config?.general.provider_indicator ?? "border";
  const warningHours = config?.general.age_warning_hours ?? 48;
  const dangerHours = config?.general.age_danger_hours ?? 144;

  const allProviders = useMemo(() => {
    if (!data) return [];
    const providers = new Set<string>();
    for (const pr of [...data.reviewing, ...data.authored]) {
      providers.add(pr.id.provider);
    }
    return [...providers].sort();
  }, [data]);

  const toggleProvider = (provider: string) => {
    setEnabledProviders((prev) => {
      const current = prev ?? new Set(allProviders);
      const next = new Set(current);
      if (next.has(provider)) {
        next.delete(provider);
      } else {
        next.add(provider);
      }
      return next;
    });
  };

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

  const activeProviders = enabledProviders ?? new Set(allProviders);
  const authoredIds = new Set(
    (data?.authored ?? []).map((pr) => prIdKey(pr.id)),
  );
  const reviewing = filterPrs(
    (data?.reviewing ?? []).filter((pr) => !authoredIds.has(prIdKey(pr.id))),
    search,
    hideDrafts,
    activeProviders,
  );
  const authored = filterPrs(
    data?.authored ?? [],
    search,
    hideDrafts,
    activeProviders,
  );
  const hasActiveFilters =
    hideDrafts || (enabledProviders !== null && enabledProviders.size < allProviders.length);

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

      <div className="flex items-center gap-2 border-b px-4 py-2">
        <div className="relative flex-1">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search PRs..."
            value={search}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              setSearch(e.target.value)
            }
            className="h-9 pl-9"
          />
        </div>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant={hasActiveFilters ? "secondary" : "ghost"}
              size="icon"
              className="h-9 w-9 shrink-0"
            >
              <SlidersHorizontal className="h-4 w-4" />
              {hasActiveFilters && (
                <span className="absolute -right-0.5 -top-0.5 h-2 w-2 rounded-full bg-primary" />
              )}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-48">
            <DropdownMenuLabel>Filters</DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuCheckboxItem
              checked={hideDrafts}
              onCheckedChange={setHideDrafts}
              onSelect={(e) => e.preventDefault()}
            >
              <FileEdit className="mr-2 h-3.5 w-3.5" />
              Hide drafts
            </DropdownMenuCheckboxItem>
            {allProviders.length > 1 && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel className="text-xs">
                  Providers
                </DropdownMenuLabel>
                {allProviders.map((provider) => (
                  <DropdownMenuCheckboxItem
                    key={provider}
                    checked={activeProviders.has(provider)}
                    onCheckedChange={() => toggleProvider(provider)}
                    onSelect={(e) => e.preventDefault()}
                  >
                    {provider}
                  </DropdownMenuCheckboxItem>
                ))}
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
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
            <PrList prs={reviewing} variant="reviewing" providerColors={providerColors} indicatorMode={indicatorMode} warningHours={warningHours} dangerHours={dangerHours} onSelect={setSelectedPrId} />
          </TabsContent>
          <TabsContent value="authored" className="m-0">
            <PrList prs={authored} variant="authored" providerColors={providerColors} indicatorMode={indicatorMode} warningHours={warningHours} dangerHours={dangerHours} onSelect={setSelectedPrId} />
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

function prIdKey(id: PrId): string {
  return `${id.provider}:${id.project}:${id.repository}:${id.number}`;
}

function filterPrs(
  prs: PullRequest[],
  search: string,
  hideDrafts: boolean,
  enabledProviders: Set<string>,
): PullRequest[] {
  let filtered = prs;

  if (hideDrafts) {
    filtered = filtered.filter((pr) => !pr.isDraft);
  }

  if (enabledProviders.size > 0) {
    filtered = filtered.filter((pr) => enabledProviders.has(pr.id.provider));
  }

  if (search.trim()) {
    const q = search.toLowerCase();
    filtered = filtered.filter(
      (pr) =>
        pr.title.toLowerCase().includes(q) ||
        pr.author.displayName.toLowerCase().includes(q) ||
        pr.repository.name.toLowerCase().includes(q) ||
        pr.repository.project.toLowerCase().includes(q) ||
        pr.sourceBranch.toLowerCase().includes(q) ||
        String(pr.id.number).includes(q),
    );
  }

  return filtered;
}
