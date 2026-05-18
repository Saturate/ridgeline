import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { api } from "../api";
import { onPollUpdate } from "../events";

export function usePollData(enabled: boolean = true) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ["poll-data"],
    queryFn: api.pollAll,
    staleTime: 30_000,
    enabled,
  });

  useEffect(() => {
    if (!enabled) return;
    const unlisten = onPollUpdate((result) => {
      queryClient.setQueryData(["poll-data"], result);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [queryClient, enabled]);

  return query;
}
