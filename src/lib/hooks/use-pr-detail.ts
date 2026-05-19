import { useQuery } from "@tanstack/react-query";
import { api } from "../api";
import type { PrId } from "../types";

export function usePrDetail(prId: PrId | null) {
  return useQuery({
    queryKey: ["pr-detail", prId],
    queryFn: () => api.getPrDetail(prId!),
    enabled: prId !== null,
    staleTime: 0,
    refetchInterval: prId !== null ? 30_000 : false,
  });
}
