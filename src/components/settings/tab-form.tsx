import { useState, useEffect } from "react";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import type { TabConfig, TabSource, TabDisplay, DraftFilter } from "@/lib/types";

interface TabFormProps {
  open: boolean;
  initial?: TabConfig;
  onSave: (tab: TabConfig) => void;
  onClose: () => void;
}

const SOURCE_OPTIONS: { value: TabSource; label: string; description: string }[] = [
  { value: "reviewing", label: "Reviewing", description: "PRs where you're assigned as a reviewer, plus PRs from your configured projects" },
  { value: "authored", label: "My PRs", description: "PRs you created" },
  { value: "all", label: "Everything", description: "All PRs from both sources, deduplicated" },
];

const DISPLAY_OPTIONS: { value: TabDisplay; label: string; description: string }[] = [
  { value: "reviewing", label: "Votes", description: "Shows individual reviewer vote badges and \"by author\" text" },
  { value: "authored", label: "Status", description: "Shows approval summary, build status, and policy badges" },
];

const DRAFT_OPTIONS: { value: DraftFilter | null; label: string; description: string }[] = [
  { value: null, label: "Default", description: "Follow the global draft filter toggle" },
  { value: "hide", label: "Hide", description: "Always hide draft PRs in this tab" },
  { value: "only", label: "Only drafts", description: "Only show draft PRs" },
  { value: "show", label: "Show all", description: "Always include drafts" },
];

const CC_TYPES = ["feat", "fix", "refactor", "perf", "docs", "test", "chore"];

function emptyTab(): TabConfig {
  return {
    label: "",
    source: "all",
    display: "reviewing",
    enabled: true,
    filter: { max_reviewers: null, drafts: null, branch_prefix: null, cc_types: [] },
  };
}

export function TabFormSheet({ open, initial, onSave, onClose }: TabFormProps) {
  const [tab, setTab] = useState<TabConfig>(initial ?? emptyTab());
  useEffect(() => {
    setTab(initial ?? emptyTab());
  }, [open]);
  const update = (patch: Partial<TabConfig>) => setTab((t) => ({ ...t, ...patch }));
  const updateFilter = (patch: Partial<TabConfig["filter"]>) =>
    setTab((t) => ({ ...t, filter: { ...t.filter, ...patch } }));
  const ccTypes = tab.filter.cc_types ?? [];

  const handleOpenChange = (isOpen: boolean) => {
    if (!isOpen) onClose();
  };

  const handleSave = () => {
    onSave(tab);
    onClose();
  };

  return (
    <Sheet open={open} onOpenChange={handleOpenChange}>
      <SheetContent className="w-[420px] sm:max-w-[420px] flex flex-col">
        <SheetHeader>
          <SheetTitle>{initial ? "Edit Tab" : "New Tab"}</SheetTitle>
        </SheetHeader>

        <ScrollArea className="flex-1 -mx-6 px-6">
          <div className="space-y-4 pb-4">
            <div>
              <label className="mb-1 block text-xs font-medium">Tab name</label>
              <Input
                placeholder="e.g. Up for grabs"
                value={tab.label}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => update({ label: e.target.value })}
              />
            </div>

            <Separator />

            <OptionGroup
              label="Show PRs from"
              description="Which pool of pull requests this tab draws from"
              options={SOURCE_OPTIONS}
              value={tab.source}
              onChange={(v) => update({ source: v })}
            />

            <Separator />

            <OptionGroup
              label="Row style"
              description="How each PR row is displayed"
              options={DISPLAY_OPTIONS}
              value={tab.display}
              onChange={(v) => update({ display: v })}
            />

            <Separator />

            <div>
              <p className="text-sm font-medium">Filters</p>
              <p className="mb-3 text-xs text-muted-foreground">
                Narrow down which PRs appear in this tab. Leave empty for no filtering.
              </p>

              <div className="space-y-3">
                <div>
                  <label className="mb-1 block text-xs font-medium">Drafts</label>
                  <div className="flex gap-1">
                    {DRAFT_OPTIONS.map((opt) => (
                      <Button
                        key={String(opt.value)}
                        size="sm"
                        variant={tab.filter.drafts === opt.value ? "default" : "outline"}
                        className="h-7 px-3 text-xs"
                        onClick={() => updateFilter({ drafts: opt.value })}
                      >
                        {opt.label}
                      </Button>
                    ))}
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {DRAFT_OPTIONS.find((o) => o.value === tab.filter.drafts)?.description}
                  </p>
                </div>

                <div>
                  <label className="mb-1 block text-xs font-medium">Max reviewers</label>
                  <Input
                    type="number"
                    placeholder="any"
                    min={0}
                    value={tab.filter.max_reviewers ?? ""}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                      updateFilter({ max_reviewers: e.target.value === "" ? null : parseInt(e.target.value) })
                    }
                  />
                  <p className="mt-1 text-xs text-muted-foreground">
                    Only show PRs with at most this many reviewers. Set to 0 for unassigned PRs.
                  </p>
                </div>

                <div>
                  <label className="mb-1 block text-xs font-medium">Branch prefix</label>
                  <Input
                    placeholder="e.g. fix/"
                    value={tab.filter.branch_prefix ?? ""}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                      updateFilter({ branch_prefix: e.target.value || null })
                    }
                  />
                  <p className="mt-1 text-xs text-muted-foreground">
                    Only show PRs whose source branch starts with this prefix.
                  </p>
                </div>

                <div>
                  <label className="mb-1 block text-xs font-medium">Conventional commit types</label>
                  <div className="flex flex-wrap gap-1">
                    {CC_TYPES.map((t) => {
                      const active = ccTypes.includes(t);
                      return (
                        <Button
                          key={t}
                          size="sm"
                          variant={active ? "default" : "outline"}
                          className="h-7 px-3 text-xs"
                          onClick={() =>
                            updateFilter({
                              cc_types: active ? ccTypes.filter((x) => x !== t) : [...ccTypes, t],
                            })
                          }
                        >
                          {t}
                        </Button>
                      );
                    })}
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {ccTypes.length === 0
                      ? "No filter active. All PRs shown regardless of commit type."
                      : `Only PRs with title matching: ${ccTypes.join(", ")}`}
                  </p>
                </div>
              </div>
            </div>
          </div>
        </ScrollArea>

        <div className="flex gap-2 pt-4 border-t">
          <Button className="flex-1" onClick={handleSave} disabled={!tab.label.trim()}>
            Save
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  );
}

function OptionGroup<T extends string>({
  label,
  description,
  options,
  value,
  onChange,
}: {
  label: string;
  description: string;
  options: { value: T; label: string; description: string }[];
  value: T;
  onChange: (v: T) => void;
}) {
  const selected = options.find((o) => o.value === value);
  return (
    <div>
      <p className="text-sm font-medium">{label}</p>
      <p className="mb-2 text-xs text-muted-foreground">{description}</p>
      <div className="flex gap-1">
        {options.map((opt) => (
          <Button
            key={opt.value}
            size="sm"
            variant={value === opt.value ? "default" : "outline"}
            className="h-7 px-3 text-xs"
            onClick={() => onChange(opt.value)}
          >
            {opt.label}
          </Button>
        ))}
      </div>
      {selected && (
        <p className="mt-1 text-xs text-muted-foreground">{selected.description}</p>
      )}
    </div>
  );
}
