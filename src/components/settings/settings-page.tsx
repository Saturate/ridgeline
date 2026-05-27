import { useState } from "react";
import { Bell, Check, Clipboard, ExternalLink, Plus, Trash2, Pencil } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { Badge } from "@/components/ui/badge";
import { useConfig, useSaveConfig } from "@/lib/hooks/use-config";
import { api } from "@/lib/api";
import { ProviderForm } from "./provider-form";
import { getProviderColorMap } from "@/lib/provider-colors";
import type { Config, NotificationConfig, ProviderConfig, ProviderIndicator } from "@/lib/types";

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

  const handleSaveGeneral = (field: string, value: number | boolean | string) => {
    saveConfig.mutate({
      ...config,
      general: { ...config.general, [field]: value },
    });
  };

  const handleSaveNotification = (field: keyof NotificationConfig, value: boolean) => {
    saveConfig.mutate({
      ...config,
      general: {
        ...config.general,
        notifications: { ...config.general.notifications, [field]: value },
      },
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
                    <span
                      className="h-3 w-3 shrink-0 rounded-full"
                      style={{ backgroundColor: getProviderColorMap(config.providers)[provider.name] }}
                    />
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
              <p className="text-sm font-medium">Age warning</p>
              <p className="text-xs text-muted-foreground">
                Show <span className="text-orange-500">orange</span> after this many hours
              </p>
            </div>
            <Input
              type="number"
              className="w-24"
              value={config.general.age_warning_hours}
              min={1}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                handleSaveGeneral(
                  "age_warning_hours",
                  parseInt(e.target.value) || 48,
                )
              }
            />
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Age danger</p>
              <p className="text-xs text-muted-foreground">
                Show <span className="text-red-500">red</span> after this many hours
              </p>
            </div>
            <Input
              type="number"
              className="w-24"
              value={config.general.age_danger_hours}
              min={1}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                handleSaveGeneral(
                  "age_danger_hours",
                  parseInt(e.target.value) || 144,
                )
              }
            />
          </div>
          <Separator />
          <NotificationToggle
            label="Show project name"
            description="Show project/repo or just repo name in PR rows"
            checked={config.general.show_project_name}
            onChange={(v) => handleSaveGeneral("show_project_name", v)}
          />
          <Separator />
          <NotificationToggle
            label="Parse conventional commits"
            description="Show type and scope badges for conventional commit titles"
            checked={config.general.parse_conventional_commits}
            onChange={(v) => handleSaveGeneral("parse_conventional_commits", v)}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-base">Notifications</CardTitle>
          <Button
            size="sm"
            variant="outline"
            onClick={() => api.testNotification()}
          >
            <Bell className="mr-1 h-3.5 w-3.5" />
            Test
          </Button>
        </CardHeader>
        <CardContent className="space-y-3">
          <NotificationToggle
            label="New pull requests"
            description="When a new PR appears in your feed"
            checked={config.general.notifications.new_pr}
            onChange={(v) => handleSaveNotification("new_pr", v)}
          />
          <Separator />
          <NotificationToggle
            label="Vote changes"
            description="When a reviewer approves, rejects, or changes their vote"
            checked={config.general.notifications.vote_changed}
            onChange={(v) => handleSaveNotification("vote_changed", v)}
          />
          <Separator />
          <NotificationToggle
            label="Waiting for author"
            description="When a reviewer requests changes on your PR"
            checked={config.general.notifications.waiting_for_author}
            onChange={(v) => handleSaveNotification("waiting_for_author", v)}
          />
          <Separator />
          <NotificationToggle
            label="Build failures"
            description="When a build fails on a PR you're involved with"
            checked={config.general.notifications.build_failed}
            onChange={(v) => handleSaveNotification("build_failed", v)}
          />
          <Separator />
          <NotificationToggle
            label="PR completed"
            description="When a PR is merged or completed"
            checked={config.general.notifications.completed}
            onChange={(v) => handleSaveNotification("completed", v)}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">Provider Indicator</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Display mode</p>
              <p className="text-xs text-muted-foreground">
                How to visually distinguish providers in the PR list
              </p>
            </div>
            <div className="flex gap-1">
              {(["border", "badge", "off"] as ProviderIndicator[]).map(
                (mode) => (
                  <Button
                    key={mode}
                    size="sm"
                    variant={
                      config.general.provider_indicator === mode
                        ? "default"
                        : "outline"
                    }
                    onClick={() =>
                      handleSaveGeneral("provider_indicator", mode)
                    }
                    className="capitalize"
                  >
                    {mode}
                  </Button>
                ),
              )}
            </div>
          </div>
          {config.general.provider_indicator !== "off" &&
            config.providers.length > 1 && (
              <>
                <Separator />
                <p className="text-xs text-muted-foreground">
                  Provider colors can be changed when editing each provider.
                </p>
              </>
            )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">MCP Integration</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-xs text-muted-foreground">
            Ridgeline can act as an MCP server, giving AI coding tools direct
            access to your pull request data via the <code className="rounded bg-muted px-1 py-0.5">--mcp-stdio</code> flag.
          </p>

          <div className="space-y-2">
            <p className="text-sm font-medium">CLI commands</p>
            <CopyBlock label="Claude Code" text="claude mcp add ridgeline ridgeline -- --mcp-stdio" />
            <CopyBlock label="Codex" text="codex mcp add ridgeline -- ridgeline --mcp-stdio" />
          </div>

          <Separator />

          <div className="space-y-2">
            <p className="text-sm font-medium">JSON config</p>
            <CopyBlock
              text={`"ridgeline": {\n  "command": "ridgeline",\n  "args": ["--mcp-stdio"]\n}`}
            />
          </div>

          <Separator />

          <div className="space-y-2">
            <p className="text-sm font-medium">Setup guides</p>
            <div className="flex flex-wrap gap-2">
              {[
                { name: "Claude Desktop", url: "https://modelcontextprotocol.io/quickstart/user" },
                { name: "Claude Code", url: "https://docs.anthropic.com/en/docs/claude-code/mcp" },
                { name: "Cursor", url: "https://cursor.com/docs/mcp" },
                { name: "VS Code Copilot", url: "https://code.visualstudio.com/docs/copilot/customization/mcp-servers" },
                { name: "Windsurf", url: "https://docs.windsurf.com/windsurf/cascade/mcp" },
                { name: "Codex", url: "https://developers.openai.com/codex/mcp" },
              ].map(({ name, url }) => (
                <Button
                  key={name}
                  size="sm"
                  variant="outline"
                  className="h-7 cursor-pointer gap-1 text-xs"
                  onClick={() => api.openUrl(url)}
                >
                  {name}
                  <ExternalLink className="h-3 w-3" />
                </Button>
              ))}
            </div>
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

function NotificationToggle({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-center justify-between">
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-muted-foreground">{description}</p>
      </div>
      <button
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors ${checked ? "bg-primary" : "bg-input"}`}
      >
        <span
          className={`inline-block h-4 w-4 rounded-full bg-background shadow-sm transition-transform ${checked ? "translate-x-4" : "translate-x-0.5"}`}
        />
      </button>
    </label>
  );
}

function CopyBlock({ text, label }: { text: string; label?: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="group relative">
      {label && (
        <p className="mb-1 text-xs text-muted-foreground">{label}</p>
      )}
      <pre className="overflow-x-auto rounded-md bg-muted p-3 pr-10 text-xs">
        {text}
      </pre>
      <Button
        size="icon"
        variant="ghost"
        className="absolute right-1 top-1 h-7 w-7 cursor-pointer opacity-0 transition-opacity group-hover:opacity-100"
        onClick={handleCopy}
      >
        {copied ? (
          <Check className="h-3.5 w-3.5 text-green-500" />
        ) : (
          <Clipboard className="h-3.5 w-3.5" />
        )}
      </Button>
    </div>
  );
}
