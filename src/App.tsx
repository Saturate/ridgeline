import { useEffect } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { toast } from "sonner";
import { AppLayout } from "./components/layout/app-layout";
import { Toaster } from "@/components/ui/sonner";
import { onChange } from "@/lib/events";
import type { Change } from "@/lib/types";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, refetchOnWindowFocus: true },
  },
});

function changeMessage(change: Change): { title: string; description: string } {
  if (change.type === "newPr") {
    return {
      title: "New Pull Request",
      description: `${change.author} opened "${change.title}" in ${change.repo}`,
    };
  }
  return {
    title: "Vote Changed",
    description: `${change.reviewer} voted on "${change.prTitle}"`,
  };
}

function ChangeNotifier() {
  useEffect(() => {
    const unlisten = onChange((change) => {
      const msg = changeMessage(change);
      toast(msg.title, { description: msg.description });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
  return null;
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AppLayout />
      <ChangeNotifier />
      <Toaster />
    </QueryClientProvider>
  );
}
