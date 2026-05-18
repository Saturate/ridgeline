import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../api";
import type { Config } from "../types";

export function useConfig() {
  return useQuery({ queryKey: ["config"], queryFn: api.getConfig });
}

export function useSaveConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (config: Config) => api.saveConfig(config),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["config"] }),
  });
}
