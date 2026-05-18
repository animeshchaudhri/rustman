import { useCallback, useEffect, useState } from "react";

import {
  createCollection as createCollectionRecord,
  deleteCollection as deleteCollectionRecord,
  deleteRequest as deleteRequestRecord,
  getCollections as getCollectionRecords,
  getRequestsForCollection,
  saveRequest as saveRequestRecord,
  updateCollection as updateCollectionRecord,
  updateRequest as updateRequestRecord,
} from "@/lib/db";
import type { Collection, SavedRequest } from "../types";

type RequestsByCollection = Record<string, SavedRequest[]>;

const cloneRequest = (request: SavedRequest): SavedRequest => ({
  ...request,
  headers: request.headers.map((header) => ({ ...header })),
  params: request.params.map((param) => ({ ...param })),
  formDataFields: request.formDataFields.map((field) => ({ ...field })),
  cookies: (request.cookies ?? []).map((c) => ({ ...c })),
});

const sortRequests = (items: SavedRequest[]) =>
  [...items].sort((a, b) => a.name.localeCompare(b.name));

export function useCollections() {
  const [collections, setCollections] = useState<Collection[]>([]);
  const [requests, setRequests] = useState<RequestsByCollection>({});

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      const collectionRecords = await getCollectionRecords();
      const requestEntries = await Promise.all(
        collectionRecords.map(async (collection) => [
          collection.id,
          await getRequestsForCollection(collection.id),
        ] as const),
      );

      if (cancelled) {
        return;
      }

      setCollections(collectionRecords);
      setRequests(Object.fromEntries(requestEntries));
    };

    void load();

    return () => {
      cancelled = true;
    };
  }, []);

  const createCollection = useCallback(async (name: string) => {
    const trimmedName = name.trim() || "New Collection";
    const collection = await createCollectionRecord(trimmedName);

    setCollections((prev) => [...prev, collection].sort((a, b) => a.createdAt - b.createdAt));
    setRequests((prev) => ({ ...prev, [collection.id]: [] }));

    return collection;
  }, []);

  const deleteCollection = useCallback(async (collectionId: string) => {
    await deleteCollectionRecord(collectionId);

    setCollections((prev) => prev.filter((collection) => collection.id !== collectionId));
    setRequests((prev) => {
      const next = { ...prev };
      delete next[collectionId];
      return next;
    });
  }, []);

  const renameCollection = useCallback(
    async (collectionId: string, name: string) => {
      const current = collections.find((collection) => collection.id === collectionId);
      if (!current) {
        return null;
      }

      const updated = await updateCollectionRecord({
        ...current,
        name: name.trim() || current.name,
      });

      setCollections((prev) =>
        prev.map((collection) =>
          collection.id === collectionId ? updated : collection,
        ),
      );

      return updated;
    },
    [collections],
  );

  const saveRequest = useCallback(async (request: Omit<SavedRequest, "id"> & { id?: string }) => {
    const normalized: SavedRequest = cloneRequest({
      ...request,
      id: request.id ?? crypto.randomUUID(),
    });

    const existingRequest = Object.values(requests)
      .flat()
      .find((item) => item.id === normalized.id);

    const saved = existingRequest
      ? await updateRequestRecord(normalized)
      : await saveRequestRecord(normalized);

    setRequests((prev) => {
      const next: RequestsByCollection = Object.fromEntries(
        Object.entries(prev).map(([collectionId, items]) => [
          collectionId,
          items.filter((item) => item.id !== saved.id),
        ]),
      );

      next[saved.collectionId] = sortRequests([...(next[saved.collectionId] ?? []), saved]);
      return next;
    });

    return saved;
  }, [requests]);

  const deleteRequest = useCallback(async (requestId: string, collectionId: string) => {
    await deleteRequestRecord(requestId);

    setRequests((prev) => ({
      ...prev,
      [collectionId]: (prev[collectionId] ?? []).filter((request) => request.id !== requestId),
    }));
  }, []);

  const renameRequest = useCallback(
    async (requestId: string, collectionId: string, name: string) => {
      const current = (requests[collectionId] ?? []).find((request) => request.id === requestId);
      if (!current) {
        return null;
      }

      const updated = await updateRequestRecord({
        ...current,
        name: name.trim() || current.name,
      });

      setRequests((prev) => ({
        ...prev,
        [collectionId]: sortRequests(
          (prev[collectionId] ?? []).map((request) =>
            request.id === requestId ? updated : request,
          ),
        ),
      }));

      return updated;
    },
    [requests],
  );

  const loadRequest = useCallback((request: SavedRequest) => cloneRequest(request), []);

  return {
    collections,
    requests,
    createCollection,
    deleteCollection,
    saveRequest,
    deleteRequest,
    loadRequest,
    renameCollection,
    renameRequest,
  };
}
