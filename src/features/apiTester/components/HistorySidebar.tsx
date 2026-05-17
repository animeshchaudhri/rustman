import type { HistoryEntry } from "../types";
import { cn } from "@/lib/utils";
import { Clock, RotateCcw, Trash2 } from "lucide-react";

const METHOD_BADGE: Record<string, string> = {
  GET: "text-emerald-400",
  POST: "text-orange-400",
  PUT: "text-blue-400",
  PATCH: "text-teal-400",
  DELETE: "text-red-400",
  HEAD: "text-purple-400",
  OPTIONS: "text-sky-400",
};

function relativeTime(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 60_000) return "just now";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  return new Date(ts).toLocaleDateString();
}

function groupByDate(entries: HistoryEntry[]): Record<string, HistoryEntry[]> {
  const groups: Record<string, HistoryEntry[]> = {};
  for (const e of entries) {
    const d = new Date(e.timestamp);
    const today = new Date();
    const yesterday = new Date(today);
    yesterday.setDate(today.getDate() - 1);

    let label: string;
    if (d.toDateString() === today.toDateString()) label = "Today";
    else if (d.toDateString() === yesterday.toDateString()) label = "Yesterday";
    else label = d.toLocaleDateString(undefined, { month: "short", day: "numeric" });

    if (!groups[label]) groups[label] = [];
    groups[label].push(e);
  }
  return groups;
}

function statusColor(status: number): string {
  if (status >= 200 && status < 300) return "text-emerald-400";
  if (status >= 300 && status < 400) return "text-sky-400";
  if (status >= 400 && status < 500) return "text-orange-400";
  if (status >= 500) return "text-red-400";
  return "text-zinc-500";
}

interface HistorySidebarProps {
  history: HistoryEntry[];
  onReplayRequest: (entry: HistoryEntry) => void;
  onClearHistory: () => void;
}

export function HistorySidebar({
  history,
  onReplayRequest,
  onClearHistory,
}: HistorySidebarProps) {
  const groups = groupByDate(history);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2.5 border-b border-zinc-700/50">
        <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">
          History
        </span>
        {history.length > 0 && (
          <button
            onClick={onClearHistory}
            className="p-1 text-zinc-500 hover:text-red-400 hover:bg-zinc-700 rounded transition-colors"
            title="Clear history"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        )}
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto py-1">
        {history.length === 0 && (
          <div className="flex flex-col items-center justify-center py-12 px-4 text-center">
            <Clock className="h-8 w-8 text-zinc-600 mb-2" />
            <p className="text-xs text-zinc-500">No history yet</p>
            <p className="text-xs text-zinc-600 mt-1">Sent requests will appear here</p>
          </div>
        )}

        {Object.entries(groups).map(([label, entries]) => (
          <div key={label}>
            <div className="px-3 py-1.5 text-[10px] font-semibold text-zinc-600 uppercase tracking-wider sticky top-0 bg-zinc-900">
              {label}
            </div>
            {entries.map((entry) => (
              <div
                key={entry.id}
                className="group flex items-start gap-2 px-2 py-2 hover:bg-zinc-800 cursor-pointer rounded-sm mx-1"
                onClick={() => onReplayRequest(entry)}
                title="Click to open in new tab"
              >
                <span
                  className={cn(
                    "text-[10px] font-bold shrink-0 pt-0.5",
                    METHOD_BADGE[entry.method] ?? "text-zinc-400",
                  )}
                >
                  {entry.method.slice(0, 3)}
                </span>

                <div className="flex-1 min-w-0">
                  <p className="text-xs text-zinc-300 truncate font-mono leading-snug">
                    {entry.url.replace(/^https?:\/\/[^/]+/, "") || "/"}
                  </p>
                  <p className="text-[10px] text-zinc-600 truncate mt-0.5 font-mono">
                    {entry.url.split("/")[2] ?? entry.url}
                  </p>
                </div>

                <div className="flex flex-col items-end gap-0.5 shrink-0">
                  {entry.status > 0 && (
                    <span className={cn("text-[10px] font-medium", statusColor(entry.status))}>
                      {entry.status}
                    </span>
                  )}
                  <span className="text-[10px] text-zinc-600">{relativeTime(entry.timestamp)}</span>
                  {entry.duration > 0 && (
                    <span className="text-[10px] text-zinc-600">{entry.duration}ms</span>
                  )}
                </div>

                <button
                  className="opacity-0 group-hover:opacity-100 shrink-0 p-0.5 text-zinc-500 hover:text-orange-400 hover:bg-zinc-700 rounded transition-all"
                  onClick={(e) => {
                    e.stopPropagation();
                    onReplayRequest(entry);
                  }}
                  title="Replay"
                >
                  <RotateCcw className="h-3 w-3" />
                </button>
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
