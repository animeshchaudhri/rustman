import { useCallback, useEffect, useState } from "react";

import {
  addToHistory as addToHistoryRecord,
  clearHistory as clearHistoryRecord,
  getHistory as getHistoryRecords,
} from "@/lib/db";
import type { HistoryEntry } from "../types";

const HISTORY_LIMIT = 100;

export function useHistory() {
  const [history, setHistory] = useState<HistoryEntry[]>([]);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      const entries = await getHistoryRecords();
      if (!cancelled) {
        setHistory(entries);
      }
    };

    void load();

    return () => {
      cancelled = true;
    };
  }, []);

  const addToHistory = useCallback(async (entry: Omit<HistoryEntry, "id" | "timestamp"> & Partial<Pick<HistoryEntry, "id" | "timestamp">>) => {
    const normalized: HistoryEntry = {
      ...entry,
      id: entry.id ?? crypto.randomUUID(),
      timestamp: entry.timestamp ?? Date.now(),
    };

    await addToHistoryRecord(normalized);
    setHistory((prev) => [normalized, ...prev.filter((item) => item.id !== normalized.id)].slice(0, HISTORY_LIMIT));
    return normalized;
  }, []);

  const clearHistory = useCallback(async () => {
    await clearHistoryRecord();
    setHistory([]);
  }, []);

  return {
    history,
    addToHistory,
    clearHistory,
  };
}
