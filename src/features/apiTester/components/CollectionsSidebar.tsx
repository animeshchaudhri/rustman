import { useRef, useState } from "react";
import type { Collection, SavedRequest } from "../types";
import { importPostmanCollection } from "../utils/postmanImport";
import { cn } from "@/lib/utils";
import {
  ChevronDown,
  ChevronRight,
  FolderOpen,
  MoreHorizontal,
  Upload,
  Pencil,
  FolderPlus,
  Plus,
  Trash2,
} from "lucide-react";

const METHOD_BADGE: Record<string, string> = {
  GET: "text-emerald-500 dark:text-emerald-400 bg-emerald-500/10",
  POST: "text-orange-500 dark:text-orange-400 bg-orange-400/10",
  PUT: "text-blue-500 dark:text-blue-400 bg-blue-400/10",
  PATCH: "text-teal-500 dark:text-teal-400 bg-teal-400/10",
  DELETE: "text-red-500 dark:text-red-400 bg-red-400/10",
  HEAD: "text-purple-500 dark:text-purple-400 bg-purple-400/10",
  OPTIONS: "text-sky-500 dark:text-sky-400 bg-sky-400/10",
};

interface CollectionsSidebarProps {
  collections: Collection[];
  requests: Record<string, SavedRequest[]>;
  onLoadRequest: (req: SavedRequest) => void;
  onCreateCollection: (name: string) => void;
  onRenameCollection: (id: string, name: string) => void;
  onDeleteCollection: (id: string) => void;
  onDeleteRequest: (id: string, collectionId: string) => void;
  onRenameRequest: (id: string, collectionId: string, name: string) => void;
  onImportCollection: (collection: Collection, requests: SavedRequest[]) => void;
}

interface MenuState {
  type: "collection" | "request";
  id: string;
  collectionId?: string;
  x: number;
  y: number;
}

export function CollectionsSidebar({
  collections,
  requests,
  onLoadRequest,
  onCreateCollection,
  onRenameCollection,
  onDeleteCollection,
  onDeleteRequest,
  onRenameRequest,
  onImportCollection,
}: CollectionsSidebarProps) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [renaming, setRenaming] = useState<{ id: string; value: string; type: "c" | "r"; collectionId?: string } | null>(null);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [newCollName, setNewCollName] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const toggleExpand = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  };

  const commitRename = () => {
    if (!renaming) return;
    const name = renaming.value.trim();
    if (renaming.type === "c" && name) onRenameCollection(renaming.id, name);
    if (renaming.type === "r" && name && renaming.collectionId)
      onRenameRequest(renaming.id, renaming.collectionId, name);
    setRenaming(null);
  };

  const handleImportFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      try {
        const json = JSON.parse(ev.target?.result as string);
        const result = importPostmanCollection(json);
        onImportCollection(result.collection, result.requests);
        setExpanded((prev) => new Set([...prev, result.collection.id]));
      } catch (err) {
        alert(`Import failed: ${err instanceof Error ? err.message : String(err)}`);
      }
    };
    reader.readAsText(file);
    e.target.value = "";
  };

  const closeMenu = () => setMenu(null);

  return (
    <div className="flex flex-col h-full" onClick={closeMenu}>
      <div className="flex items-center justify-between px-3 py-2.5 border-b border-stone-300/50 dark:border-zinc-700/50">
        <span className="text-xs font-semibold text-zinc-500 dark:text-zinc-400 uppercase tracking-wider">
          Collections
        </span>
        <div className="flex gap-1">
          <button
            onClick={(e) => {
              e.stopPropagation();
              fileRef.current?.click();
            }}
            className="p-1 text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-200 dark:hover:bg-zinc-700 rounded transition-colors"
            title="Import Postman Collection"
          >
            <Upload className="h-3.5 w-3.5" />
          </button>
          <button
            onClick={(e) => {
              e.stopPropagation();
              setNewCollName("");
            }}
            className="p-1 text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-200 dark:hover:bg-zinc-700 rounded transition-colors"
            title="New Collection"
          >
            <FolderPlus className="h-3.5 w-3.5" />
          </button>
        </div>
        <input ref={fileRef} type="file" accept=".json" className="hidden" onChange={handleImportFile} />
      </div>

      <div className="flex-1 overflow-y-auto py-1">
        {newCollName !== null && (
          <div className="px-2 py-1">
            <input
              autoFocus
              value={newCollName}
              onChange={(e) => setNewCollName(e.target.value)}
              onBlur={() => {
                if (newCollName.trim()) onCreateCollection(newCollName.trim());
                setNewCollName(null);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  if (newCollName.trim()) onCreateCollection(newCollName.trim());
                  setNewCollName(null);
                }
                if (e.key === "Escape") setNewCollName(null);
              }}
              placeholder="Collection name…"
              className="w-full bg-white dark:bg-zinc-700 border border-orange-500/50 rounded px-2 py-1 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none placeholder:text-zinc-500 dark:placeholder:text-zinc-500"
            />
          </div>
        )}

        {collections.length === 0 && newCollName === null && (
          <div className="flex flex-col items-center justify-center py-12 px-4 text-center">
            <FolderOpen className="h-8 w-8 text-zinc-400 dark:text-zinc-600 mb-2" />
            <p className="text-xs text-zinc-500 dark:text-zinc-500">No collections yet</p>
            <p className="text-xs text-zinc-400 dark:text-zinc-600 mt-1">
              Create one or import a Postman collection
            </p>
          </div>
        )}

        {collections.map((col) => {
          const isOpen = expanded.has(col.id);
          const reqs = requests[col.id] ?? [];

          return (
            <div key={col.id}>
              <div
                className="group flex items-center gap-1.5 px-2 py-1.5 hover:bg-stone-100 dark:hover:bg-zinc-800 cursor-pointer select-none"
                onClick={() => toggleExpand(col.id)}
              >
                <span className="text-zinc-500 dark:text-zinc-500 shrink-0">
                  {isOpen ? (
                    <ChevronDown className="h-3.5 w-3.5" />
                  ) : (
                    <ChevronRight className="h-3.5 w-3.5" />
                  )}
                </span>

                {renaming?.id === col.id && renaming.type === "c" ? (
                  <input
                    autoFocus
                    value={renaming.value}
                    onChange={(e) => setRenaming({ ...renaming, value: e.target.value })}
                    onBlur={commitRename}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") commitRename();
                      if (e.key === "Escape") setRenaming(null);
                    }}
                    onClick={(e) => e.stopPropagation()}
                    className="flex-1 bg-white dark:bg-zinc-700 border border-orange-500/50 rounded px-1.5 py-0.5 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none"
                  />
                ) : (
                  <span className="flex-1 text-xs text-zinc-700 dark:text-zinc-300 truncate font-medium">
                    {col.name}
                  </span>
                )}

                <span className="text-xs text-zinc-400 dark:text-zinc-600 shrink-0">{reqs.length}</span>

                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    const rect = e.currentTarget.getBoundingClientRect();
                    setMenu({ type: "collection", id: col.id, x: rect.right, y: rect.bottom });
                  }}
                  className="opacity-0 group-hover:opacity-100 p-0.5 text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-200 dark:hover:bg-zinc-700 rounded transition-all shrink-0"
                >
                  <MoreHorizontal className="h-3.5 w-3.5" />
                </button>
              </div>

              {isOpen && (
                <div className="ml-4 border-l border-stone-300/40 dark:border-zinc-700/40 pl-1">
                  {reqs.length === 0 && (
                    <p className="text-xs text-zinc-400 dark:text-zinc-600 px-3 py-2 italic">No requests</p>
                  )}
                  {reqs.map((req) => (
                    <div
                      key={req.id}
                      className="group flex items-center gap-2 px-2 py-1.5 hover:bg-stone-100 dark:hover:bg-zinc-800 cursor-pointer rounded-sm mx-1"
                      onClick={() => onLoadRequest(req)}
                    >
                      <span
                        className={cn(
                          "text-[10px] font-bold rounded px-1 py-0.5 shrink-0 leading-none",
                          METHOD_BADGE[req.method] ?? "text-zinc-500 dark:text-zinc-400 bg-stone-200 dark:bg-zinc-700",
                        )}
                      >
                        {req.method.slice(0, 3)}
                      </span>

                      {renaming?.id === req.id && renaming.type === "r" ? (
                        <input
                          autoFocus
                          value={renaming.value}
                          onChange={(e) => setRenaming({ ...renaming, value: e.target.value })}
                          onBlur={commitRename}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") commitRename();
                            if (e.key === "Escape") setRenaming(null);
                          }}
                          onClick={(e) => e.stopPropagation()}
                          className="flex-1 bg-white dark:bg-zinc-700 border border-orange-500/50 rounded px-1.5 py-0.5 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none"
                        />
                      ) : (
                        <span className="flex-1 text-xs text-zinc-500 dark:text-zinc-400 truncate">{req.name}</span>
                      )}

                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          const rect = e.currentTarget.getBoundingClientRect();
                          setMenu({
                            type: "request",
                            id: req.id,
                            collectionId: col.id,
                            x: rect.right,
                            y: rect.bottom,
                          });
                        }}
                        className="opacity-0 group-hover:opacity-100 p-0.5 text-zinc-400 dark:text-zinc-600 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-200 dark:hover:bg-zinc-700 rounded transition-all shrink-0"
                      >
                        <MoreHorizontal className="h-3 w-3" />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {menu && (
        <>
          <div className="fixed inset-0 z-40" onClick={closeMenu} />
          <div
            className="fixed z-50 bg-white dark:bg-zinc-800 border border-stone-300 dark:border-zinc-700 rounded-lg shadow-xl py-1 min-w-[160px]"
            style={{ left: menu.x - 160, top: menu.y + 4 }}
            onClick={(e) => e.stopPropagation()}
          >
            {menu.type === "collection" && (
              <>
                <button
                  className="flex items-center gap-2.5 w-full px-3 py-2 text-xs text-zinc-700 dark:text-zinc-300 hover:bg-stone-100 dark:hover:bg-zinc-700 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
                  onClick={() => {
                    const col = collections.find((c) => c.id === menu.id);
                    if (col) setRenaming({ id: col.id, value: col.name, type: "c" });
                    closeMenu();
                  }}
                >
                  <Pencil className="h-3.5 w-3.5" />
                  Rename
                </button>
                <div className="h-px bg-stone-200 dark:bg-zinc-700 my-1" />
                <button
                  className="flex items-center gap-2.5 w-full px-3 py-2 text-xs text-red-500 dark:text-red-400 hover:bg-red-500/10 transition-colors"
                  onClick={() => {
                    onDeleteCollection(menu.id);
                    closeMenu();
                  }}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                  Delete collection
                </button>
              </>
            )}
            {menu.type === "request" && menu.collectionId && (
              <>
                <button
                  className="flex items-center gap-2.5 w-full px-3 py-2 text-xs text-zinc-700 dark:text-zinc-300 hover:bg-stone-100 dark:hover:bg-zinc-700 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
                  onClick={() => {
                    const col = menu.collectionId!;
                    const req = (requests[col] ?? []).find((r) => r.id === menu.id);
                    if (req) setRenaming({ id: req.id, value: req.name, type: "r", collectionId: col });
                    closeMenu();
                  }}
                >
                  <Pencil className="h-3.5 w-3.5" />
                  Rename
                </button>
                <button
                  className="flex items-center gap-2.5 w-full px-3 py-2 text-xs text-zinc-700 dark:text-zinc-300 hover:bg-stone-100 dark:hover:bg-zinc-700 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
                  onClick={() => {
                    const req = (requests[menu.collectionId!] ?? []).find((r) => r.id === menu.id);
                    if (req) onLoadRequest(req);
                    closeMenu();
                  }}
                >
                  <Plus className="h-3.5 w-3.5" />
                  Open in tab
                </button>
                <div className="h-px bg-stone-200 dark:bg-zinc-700 my-1" />
                <button
                  className="flex items-center gap-2.5 w-full px-3 py-2 text-xs text-red-500 dark:text-red-400 hover:bg-red-500/10 transition-colors"
                  onClick={() => {
                    onDeleteRequest(menu.id, menu.collectionId!);
                    closeMenu();
                  }}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                  Delete request
                </button>
              </>
            )}
          </div>
        </>
      )}
    </div>
  );
}
