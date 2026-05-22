import { useState, useEffect, useCallback } from "react";
import { useQuery } from "@tanstack/react-query";
import { Settings, RefreshCw, Mountain } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dashboard } from "@/components/layout/dashboard";
import { SettingsPage } from "@/components/settings/settings-page";
import { api } from "@/lib/api";
import { useConfig } from "@/lib/hooks/use-config";

type View = "dashboard" | "settings";

const DEMO_MODE = window.location.search.includes("demo");

export function AppLayout() {
  const [view, setView] = useState<View>("dashboard");
  const [initialized, setInitialized] = useState(DEMO_MODE);
  const [initError, setInitError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const config = useConfig();
  const { data: version } = useQuery({ queryKey: ["version"], queryFn: api.getVersion });

  const initialize = useCallback(async () => {
    try {
      await api.initProviders();
      setInitialized(true);
      setInitError(null);
      api.startPolling();
    } catch (e) {
      setInitError(String(e));
      setInitialized(false);
    }
  }, []);

  useEffect(() => {
    if (
      config.data &&
      config.data.providers.length > 0 &&
      !initialized &&
      !initError
    ) {
      initialize();
    }
  }, [config.data, initialized, initError, initialize]);

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await initialize();
    } finally {
      setRefreshing(false);
    }
  };

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key === "r") {
        e.preventDefault();
        handleRefresh();
      }
      if (mod && e.key === ",") {
        e.preventDefault();
        setView((v) => (v === "settings" ? "dashboard" : "settings"));
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  useEffect(() => {
    const prefersDark = window.matchMedia(
      "(prefers-color-scheme: dark)",
    ).matches;
    document.documentElement.classList.toggle("dark", prefersDark);

    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = (e: MediaQueryListEvent) => {
      document.documentElement.classList.toggle("dark", e.matches);
    };
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  const needsSetup =
    config.data && config.data.providers.length === 0 && !initialized;

  return (
    <div className="flex h-screen flex-col">
      <header className="flex h-12 items-center justify-between border-b px-4">
        <div className="flex items-center gap-2">
          <Mountain className="h-5 w-5 text-primary" />
          <h1 className="text-sm font-semibold">Ridgeline</h1>
          {version && (
            <span className="text-xs text-muted-foreground">v{version}</span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={handleRefresh}
            disabled={refreshing}
          >
            <RefreshCw
              className={`h-4 w-4 ${refreshing ? "animate-spin" : ""}`}
            />
          </Button>
          <Button
            variant={view === "settings" ? "secondary" : "ghost"}
            size="icon"
            className="h-8 w-8"
            onClick={() =>
              setView((v) => (v === "settings" ? "dashboard" : "settings"))
            }
          >
            <Settings className="h-4 w-4" />
          </Button>
        </div>
      </header>

      <main className="flex-1 overflow-y-auto">
        {view === "settings" || needsSetup ? (
          <SettingsPage
            onDone={() => {
              setInitialized(false);
              setInitError(null);
              setView("dashboard");
            }}
          />
        ) : (
          <Dashboard initialized={initialized} initError={initError} onRetry={handleRefresh} />
        )}
      </main>
    </div>
  );
}
