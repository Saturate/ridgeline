import { Clock } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { relativeTime, isStale } from "@/lib/time";

interface AgeIndicatorProps {
  createdAt: string;
  staleThresholdHours?: number;
}

export function AgeIndicator({
  createdAt,
  staleThresholdHours = 48,
}: AgeIndicatorProps) {
  const age = relativeTime(createdAt);
  const stale = isStale(createdAt, staleThresholdHours);

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            className={cn(
              "inline-flex items-center gap-1 text-xs",
              stale
                ? "text-orange-600 dark:text-orange-400"
                : "text-muted-foreground",
            )}
          >
            <Clock className="h-3 w-3" />
            {age}
          </span>
        </TooltipTrigger>
        <TooltipContent>
          Created {new Date(createdAt).toLocaleString()}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
