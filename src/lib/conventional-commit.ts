export interface ConventionalCommit {
  type: string;
  scope: string | null;
  description: string;
  breaking: boolean;
}

const CC_REGEX = /^(\w+)(?:\(([^)]+)\))?(!?):\s*(.+)$/;

const TYPE_COLORS: Record<string, string> = {
  feat: "bg-green-500/20 text-green-700 dark:text-green-400",
  fix: "bg-red-500/20 text-red-700 dark:text-red-400",
  refactor: "bg-purple-500/20 text-purple-700 dark:text-purple-400",
  perf: "bg-orange-500/20 text-orange-700 dark:text-orange-400",
  docs: "bg-blue-500/20 text-blue-700 dark:text-blue-400",
  test: "bg-cyan-500/20 text-cyan-700 dark:text-cyan-400",
  chore: "bg-muted text-muted-foreground",
  ci: "bg-muted text-muted-foreground",
  style: "bg-muted text-muted-foreground",
  build: "bg-muted text-muted-foreground",
};

export function parseConventionalCommit(
  title: string,
): ConventionalCommit | null {
  const match = title.match(CC_REGEX);
  if (!match) return null;
  return {
    type: match[1]!,
    scope: match[2] ?? null,
    description: match[4]!,
    breaking: match[3] === "!",
  };
}

export function getTypeColor(type: string): string {
  return TYPE_COLORS[type] ?? "bg-muted text-muted-foreground";
}
