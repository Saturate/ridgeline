import type { ProviderConfig } from "./types";

const DEFAULT_COLORS = [
  "#3b82f6",
  "#10b981",
  "#f59e0b",
  "#ef4444",
  "#8b5cf6",
  "#ec4899",
  "#06b6d4",
  "#f97316",
];

export function getProviderColor(
  providerName: string,
  providers: ProviderConfig[],
): string | undefined {
  const provider = providers.find((p) => p.name === providerName);
  if (provider?.color) return provider.color;

  const index = providers.findIndex((p) => p.name === providerName);
  if (index >= 0) return DEFAULT_COLORS[index % DEFAULT_COLORS.length];
  return undefined;
}

export function getProviderColorMap(
  providers: ProviderConfig[],
): Record<string, string> {
  const map: Record<string, string> = {};
  for (const provider of providers) {
    map[provider.name] = provider.color ?? DEFAULT_COLORS[providers.indexOf(provider) % DEFAULT_COLORS.length] ?? "#3b82f6";
  }
  return map;
}
