import { useState, useEffect, useCallback, useRef, useMemo } from "react";
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
import { WifiOff, KeyRound, ServerCrash, Mountain, ChevronDown, ChevronUp, RefreshCw, Search, SlidersHorizontal, FileEdit } from "lucide-react";
import { Skeleton } from "@/components/ui/skeleton";
import { usePollData } from "@/lib/hooks/use-poll-data";
import { PrList } from "@/components/pr/pr-list";
import { PrDetailPanel } from "@/components/pr/pr-detail-panel";
import { useConfig } from "@/lib/hooks/use-config";
import { getProviderColorMap } from "@/lib/provider-colors";
import { parseConventionalCommit } from "@/lib/conventional-commit";
import type { PrId, PollError, PollErrorKind, PullRequest, TabConfig, TabSource } from "@/lib/types";

interface DashboardProps {
  initialized: boolean;
  initError: string | null;
  onRetry?: () => void;
}

export function Dashboard({ initialized, initError, onRetry }: DashboardProps) {
  const [search, setSearch] = useState("");
  const [selectedPrId, setSelectedPrId] = useState<PrId | null>(null);
  const [hideDrafts, setHideDrafts] = useState(true);
  const [enabledProviders, setEnabledProviders] = useState<Set<string> | null>(
    null,
  );
  const { data, isLoading, error, refetch } = usePollData(initialized);
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
  const showProjectName = config?.general.show_project_name ?? true;
  const parseCC = config?.general.parse_conventional_commits ?? false;

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
    return <FullScreenError message={initError} onRetry={onRetry} />;
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
    return <FullScreenError message={String(error)} onRetry={() => refetch()} />;
  }

  const activeProviders = enabledProviders ?? new Set(allProviders);
  const authoredIds = new Set(
    (data?.authored ?? []).map((pr) => prIdKey(pr.id)),
  );

  const emptyFilter = { max_reviewers: null, drafts: null, branch_prefix: null, cc_types: [] };
  const defaultTabs: TabConfig[] = [
    { label: "Reviewing", source: "reviewing", display: "reviewing", enabled: true, filter: emptyFilter },
    { label: "Authored", source: "authored", display: "authored", enabled: true, filter: emptyFilter },
  ];
  const tabs = (config?.general.tabs?.filter((t) => t.enabled) ?? []).length > 0
    ? config!.general.tabs.filter((t) => t.enabled)
    : defaultTabs;

  const getPrsForSource = (source: TabSource): PullRequest[] => {
    switch (source) {
      case "reviewing":
        return (data?.reviewing ?? []).filter((pr) => !authoredIds.has(prIdKey(pr.id)));
      case "authored":
        return data?.authored ?? [];
      case "all": {
        const seen = new Set<string>();
        const combined: PullRequest[] = [];
        for (const pr of [...(data?.reviewing ?? []), ...(data?.authored ?? [])]) {
          const key = prIdKey(pr.id);
          if (!seen.has(key)) {
            seen.add(key);
            combined.push(pr);
          }
        }
        return combined;
      }
    }
  };

  const applyTabFilter = (prs: PullRequest[], tab: TabConfig): PullRequest[] => {
    let filtered = prs;
    if (tab.filter.max_reviewers != null) {
      filtered = filtered.filter((pr) => pr.reviewers.length <= tab.filter.max_reviewers!);
    }
    if (tab.filter.branch_prefix) {
      const prefix = tab.filter.branch_prefix.toLowerCase();
      filtered = filtered.filter((pr) => pr.sourceBranch.toLowerCase().startsWith(prefix));
    }
    if (tab.filter.cc_types?.length > 0) {
      const types = new Set(tab.filter.cc_types.map((t) => t.toLowerCase()));
      filtered = filtered.filter((pr) => {
        const cc = parseConventionalCommit(pr.title);
        return cc !== null && types.has(cc.type.toLowerCase());
      });
    }
    return filtered;
  };

  const tabData = tabs.map((tab, i) => {
    const sourcePrs = getPrsForSource(tab.source);
    const tabFiltered = applyTabFilter(sourcePrs, tab);
    let draftFiltered = tabFiltered;
    if (tab.filter.drafts === "hide") {
      draftFiltered = draftFiltered.filter((pr) => !pr.isDraft);
    } else if (tab.filter.drafts === "only") {
      draftFiltered = draftFiltered.filter((pr) => pr.isDraft);
    } else if (tab.filter.drafts === "show") {
      // show all, skip global filter
    }
    const effectiveHideDrafts = tab.filter.drafts ? false : hideDrafts;
    const prs = filterPrs(draftFiltered, search, effectiveHideDrafts, activeProviders);
    return { tab, prs, key: `${tab.label}-${i}` };
  });

  const hasActiveFilters =
    hideDrafts || (enabledProviders !== null && enabledProviders.size < allProviders.length);

  return (
    <div className="flex h-full flex-col">
      {data?.errors && data.errors.length > 0 && (
        <ErrorBanner errors={data.errors} />
      )}

      <div className="sticky top-0 z-10 flex items-center gap-2 border-b bg-background px-4 py-2">
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

      <Tabs defaultValue={tabData[0]?.key} className="flex flex-1 flex-col">
        <div className="sticky top-[52px] z-10 border-b bg-background px-4">
          <TabsList className="h-9">
            {tabData.map(({ tab, prs, key }) => (
              <TabsTrigger key={key} value={key} className="gap-1.5 text-xs">
                {tab.label}
                <Badge variant="secondary" className="h-5 px-1.5 text-xs">
                  {prs.length}
                </Badge>
              </TabsTrigger>
            ))}
          </TabsList>
        </div>

        <ScrollArea className="flex-1">
          {tabData.map(({ tab, prs, key }) => (
            <TabsContent key={key} value={key} className="m-0">
              <PrList prs={prs} variant={tab.display} providerColors={providerColors} indicatorMode={indicatorMode} warningHours={warningHours} dangerHours={dangerHours} showProjectName={showProjectName} parseCC={parseCC} onSelect={setSelectedPrId} />
            </TabsContent>
          ))}
        </ScrollArea>
      </Tabs>

      <PrDetailPanel
        prId={selectedPrId}
        onClose={() => setSelectedPrId(null)}
      />
    </div>
  );
}

function errorIcon(kind: PollErrorKind) {
  switch (kind) {
    case "network":
      return <WifiOff className="h-4 w-4" />;
    case "auth":
      return <KeyRound className="h-4 w-4" />;
    case "server":
      return <ServerCrash className="h-4 w-4" />;
    default:
      return <Mountain className="h-4 w-4" />;
  }
}

function friendlyTitle(kind: PollErrorKind): string {
  switch (kind) {
    case "network":
      return "Couldn’t reach the server";
    case "auth":
      return "Authentication issue";
    case "server":
      return "Server hiccup";
    case "parse":
      return "Unexpected response";
    default:
      return "Something went wrong";
  }
}

function ErrorBanner({ errors }: { errors: PollError[] }) {
  const [expanded, setExpanded] = useState(false);
  const primaryKind = errors[0]?.kind ?? "unknown";
  const providers = [...new Set(errors.map((e) => e.provider))].join(", ");

  return (
    <div className="border-b border-border/50 px-4 py-2.5">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex w-full items-center gap-2 text-left text-muted-foreground transition-colors hover:text-foreground"
      >
        {errorIcon(primaryKind)}
        <span className="flex-1 text-sm">
          {friendlyTitle(primaryKind)}
          <span className="ml-1 text-muted-foreground/60">— {providers}</span>
        </span>
        {expanded ? (
          <ChevronUp className="h-3.5 w-3.5" />
        ) : (
          <ChevronDown className="h-3.5 w-3.5" />
        )}
      </button>
      {expanded && (
        <div className="mt-2 space-y-1 pl-6">
          {errors.map((err, i) => (
            <p key={i} className="text-xs text-muted-foreground/60 break-all">
              {err.message}
            </p>
          ))}
        </div>
      )}
    </div>
  );
}

const RETRY_INTERVAL = 30;
const MAX_RETRIES = 10;

function FullScreenError({
  message,
  onRetry,
}: {
  message: string;
  onRetry?: () => void;
}) {
  const [showDetails, setShowDetails] = useState(false);
  const [countdown, setCountdown] = useState(RETRY_INTERVAL);
  const [attempt, setAttempt] = useState(1);
  const timerRef = useRef<ReturnType<typeof setInterval>>(null);

  const doRetry = useCallback(() => {
    if (attempt >= MAX_RETRIES) return;
    setAttempt((a) => a + 1);
    setCountdown(RETRY_INTERVAL);
    onRetry?.();
  }, [attempt, onRetry]);

  useEffect(() => {
    if (attempt >= MAX_RETRIES) return;
    timerRef.current = setInterval(() => {
      setCountdown((c) => {
        if (c <= 1) {
          doRetry();
          return RETRY_INTERVAL;
        }
        return c - 1;
      });
    }, 1000);
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [attempt, doRetry]);

  const handleManualRetry = () => {
    setCountdown(RETRY_INTERVAL);
    doRetry();
  };

  return (
    <div className="flex flex-col items-center justify-center gap-3 p-16 text-muted-foreground">
      <Mountain className="h-16 w-16 opacity-15" strokeWidth={1} />
      <p className="text-sm font-medium">Lost in the clouds</p>
      <p className="max-w-sm text-center text-xs text-muted-foreground/60">
        Ridgeline couldn't connect — check your network or token if this
        persists.
      </p>
      {attempt < MAX_RETRIES ? (
        <p className="text-xs text-muted-foreground/40">
          Retrying in {countdown}s ({attempt}/{MAX_RETRIES})
        </p>
      ) : (
        <p className="text-xs text-muted-foreground/40">Gave up after {MAX_RETRIES} attempts</p>
      )}
      {onRetry && (
        <Button
          variant="outline"
          size="sm"
          className="mt-1"
          onClick={handleManualRetry}
        >
          <RefreshCw className="mr-1.5 h-3.5 w-3.5" />
          Retry now
        </Button>
      )}
      <button
        onClick={() => setShowDetails(!showDetails)}
        className="mt-1 text-xs text-muted-foreground/40 underline decoration-muted-foreground/20 transition-colors hover:text-muted-foreground/60"
      >
        {showDetails ? "Hide details" : "Technical details"}
      </button>
      {showDetails && (
        <p className="mt-1 max-w-lg break-all text-center text-xs text-muted-foreground/40">
          {message}
        </p>
      )}
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
