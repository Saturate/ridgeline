import { Clock, Hourglass } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { relativeTime } from "@/lib/time";
import type { PrListVariant } from "./pr-list";

interface AgeIndicatorProps {
  createdAt: string;
  variant: PrListVariant;
  warningHours?: number;
  dangerHours?: number;
}

function ageColor(
  createdAt: string,
  warningHours: number,
  dangerHours: number,
): string {
  const hours = (Date.now() - new Date(createdAt).getTime()) / 3_600_000;
  if (hours >= dangerHours) return "text-red-600 dark:text-red-400";
  if (hours >= warningHours) return "text-orange-600 dark:text-orange-400";
  return "text-muted-foreground";
}

export function AgeIndicator({
  createdAt,
  variant,
  warningHours = 48,
  dangerHours = 144,
}: AgeIndicatorProps) {
  const age = relativeTime(createdAt);
  const Icon = variant === "reviewing" ? Hourglass : Clock;
  const color = ageColor(createdAt, warningHours, dangerHours);

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <span className={cn("inline-flex items-center gap-1 text-xs", color)}>
            <Icon className="h-3 w-3" />
            {age}
          </span>
        </TooltipTrigger>
        <TooltipContent>
          {variant === "reviewing"
            ? `Waiting for review since ${new Date(createdAt).toLocaleString()}`
            : `Open since ${new Date(createdAt).toLocaleString()}`}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
