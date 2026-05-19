import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { api } from "../api";
import { onPollUpdate } from "../events";
import { getMockData } from "../mock-data";

const DEMO_MODE = window.location.search.includes("demo");

export function usePollData(enabled: boolean = true) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ["poll-data"],
    queryFn: DEMO_MODE ? () => Promise.resolve(getMockData()) : api.pollAll,
    staleTime: 30_000,
    enabled: DEMO_MODE || enabled,
  });

  useEffect(() => {
    if (!enabled || DEMO_MODE) return;
    const unlisten = onPollUpdate((result) => {
      queryClient.setQueryData(["poll-data"], result);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [queryClient, enabled]);

  return query;
}
