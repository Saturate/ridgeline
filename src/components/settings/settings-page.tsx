import { useState } from "react";
import { Plus, Trash2, Pencil } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { Badge } from "@/components/ui/badge";
import { useConfig, useSaveConfig } from "@/lib/hooks/use-config";
import { ProviderForm } from "./provider-form";
import type { Config, ProviderConfig } from "@/lib/types";

interface SettingsPageProps {
  onDone: () => void;
}

export function SettingsPage({ onDone }: SettingsPageProps) {
  const { data: config, isLoading } = useConfig();
  const saveConfig = useSaveConfig();
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [adding, setAdding] = useState(false);

  if (isLoading || !config) return null;

  const isFirstRun = config.providers.length === 0 && !adding;

  const handleSaveProvider = (provider: ProviderConfig, index: number | null) => {
    const newProviders = [...config.providers];
    if (index !== null) {
      newProviders[index] = provider;
    } else {
      newProviders.push(provider);
    }
    const newConfig: Config = { ...config, providers: newProviders };
    saveConfig.mutate(newConfig, {
      onSuccess: () => {
        setEditingIndex(null);
        setAdding(false);
      },
    });
  };

  const handleDeleteProvider = (index: number) => {
    const newProviders = config.providers.filter((_, i) => i !== index);
    saveConfig.mutate({ ...config, providers: newProviders });
  };

  const handleSaveGeneral = (field: string, value: number | boolean) => {
    saveConfig.mutate({
      ...config,
      general: { ...config.general, [field]: value },
    });
  };

  return (
    <div className="mx-auto max-w-2xl space-y-6 p-6">
      {isFirstRun && (
        <div className="text-center">
          <h2 className="text-xl font-semibold">Welcome to Ridgeline</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Add a provider to start monitoring pull requests.
          </p>
        </div>
      )}

      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-base">Providers</CardTitle>
          {!adding && editingIndex === null && (
            <Button size="sm" variant="outline" onClick={() => setAdding(true)}>
              <Plus className="mr-1 h-4 w-4" />
              Add Provider
            </Button>
          )}
        </CardHeader>
        <CardContent className="space-y-3">
          {config.providers.map((provider, i) =>
            editingIndex === i ? (
              <ProviderForm
                key={i}
                initial={provider}
                onSave={(p) => handleSaveProvider(p, i)}
                onCancel={() => setEditingIndex(null)}
              />
            ) : (
              <div
                key={i}
                className="flex items-center justify-between rounded-md border p-3"
              >
                <div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">{provider.name}</span>
                    <Badge variant="secondary" className="text-xs">
                      Azure DevOps
                    </Badge>
                  </div>
                  <p className="mt-0.5 text-xs text-muted-foreground">
                    {provider.url}
                    {provider.projects.length > 0 &&
                      ` · ${provider.projects.length} project(s)`}
                  </p>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={() => setEditingIndex(i)}
                  >
                    <Pencil className="h-3.5 w-3.5" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-destructive"
                    onClick={() => handleDeleteProvider(i)}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            ),
          )}
          {(adding || isFirstRun) && (
            <ProviderForm
              onSave={(p) => handleSaveProvider(p, null)}
              onCancel={() => setAdding(false)}
            />
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">General</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Refresh interval</p>
              <p className="text-xs text-muted-foreground">
                How often to poll for updates (seconds)
              </p>
            </div>
            <Input
              type="number"
              className="w-24"
              value={config.general.refresh_interval_secs}
              min={10}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                handleSaveGeneral(
                  "refresh_interval_secs",
                  parseInt(e.target.value) || 60,
                )
              }
            />
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Stale threshold</p>
              <p className="text-xs text-muted-foreground">
                Mark PRs as stale after this many hours
              </p>
            </div>
            <Input
              type="number"
              className="w-24"
              value={config.general.stale_threshold_hours}
              min={1}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                handleSaveGeneral(
                  "stale_threshold_hours",
                  parseInt(e.target.value) || 48,
                )
              }
            />
          </div>
        </CardContent>
      </Card>

      {config.providers.length > 0 && (
        <div className="flex justify-end">
          <Button onClick={onDone}>Done</Button>
        </div>
      )}
    </div>
  );
}
