import { useState } from "react";
import type { Collection } from "../types";
import { cn } from "@/lib/utils";
import { FolderPlus, Save, X } from "lucide-react";

interface SaveRequestDialogProps {
  open: boolean;
  defaultName: string;
  collections: Collection[];
  onSave: (name: string, collectionId: string) => void;
  onClose: () => void;
  onCreateCollection: (name: string) => Promise<Collection>;
}

export function SaveRequestDialog({
  open,
  defaultName,
  collections,
  onSave,
  onClose,
  onCreateCollection,
}: SaveRequestDialogProps) {
  const [name, setName] = useState(defaultName);
  const [collectionId, setCollectionId] = useState(collections[0]?.id ?? "");
  const [newCollName, setNewCollName] = useState("");
  const [creatingNew, setCreatingNew] = useState(false);
  const [busy, setBusy] = useState(false);

  if (!open) return null;

  const handleSave = async () => {
    let targetId = collectionId;

    if (creatingNew) {
      if (!newCollName.trim()) return;
      setBusy(true);
      try {
        const col = await onCreateCollection(newCollName.trim());
        targetId = col.id;
      } finally {
        setBusy(false);
      }
    }

    if (!name.trim() || !targetId) return;
    onSave(name.trim(), targetId);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />

      {/* Dialog */}
      <div className="relative bg-zinc-900 border border-zinc-700 rounded-xl shadow-2xl w-[420px] p-6">
        <button
          onClick={onClose}
          className="absolute top-4 right-4 text-zinc-500 hover:text-zinc-200 transition-colors"
        >
          <X className="h-4 w-4" />
        </button>

        <h2 className="text-sm font-semibold text-zinc-100 mb-4 flex items-center gap-2">
          <Save className="h-4 w-4 text-orange-400" />
          Save Request
        </h2>

        {/* Request name */}
        <div className="mb-4">
          <label className="text-xs text-zinc-400 mb-1.5 block font-medium">Request Name</label>
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSave()}
            className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-orange-500/60 placeholder:text-zinc-600"
          />
        </div>

        {/* Collection */}
        <div className="mb-5">
          <label className="text-xs text-zinc-400 mb-1.5 block font-medium">Save to Collection</label>

          <div className="flex gap-2 mb-2">
            <button
              onClick={() => setCreatingNew(false)}
              className={cn(
                "flex-1 text-xs py-1.5 rounded-md border transition-colors",
                !creatingNew
                  ? "bg-orange-600/20 border-orange-600/40 text-orange-400"
                  : "bg-zinc-800 border-zinc-700 text-zinc-400 hover:text-zinc-200",
              )}
            >
              Existing
            </button>
            <button
              onClick={() => setCreatingNew(true)}
              className={cn(
                "flex-1 text-xs py-1.5 rounded-md border transition-colors flex items-center justify-center gap-1",
                creatingNew
                  ? "bg-orange-600/20 border-orange-600/40 text-orange-400"
                  : "bg-zinc-800 border-zinc-700 text-zinc-400 hover:text-zinc-200",
              )}
            >
              <FolderPlus className="h-3 w-3" />
              New
            </button>
          </div>

          {creatingNew ? (
            <input
              autoFocus
              value={newCollName}
              onChange={(e) => setNewCollName(e.target.value)}
              placeholder="Collection name…"
              className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-orange-500/60 placeholder:text-zinc-600"
            />
          ) : collections.length === 0 ? (
            <p className="text-xs text-zinc-500 italic px-1">
              No collections yet. Switch to "New" to create one.
            </p>
          ) : (
            <select
              value={collectionId}
              onChange={(e) => setCollectionId(e.target.value)}
              className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-300 focus:outline-none focus:border-orange-500/60 appearance-none cursor-pointer"
            >
              {collections.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          )}
        </div>

        {/* Actions */}
        <div className="flex gap-2">
          <button
            onClick={handleSave}
            disabled={busy || !name.trim() || (creatingNew ? !newCollName.trim() : !collectionId)}
            className="flex-1 bg-orange-600 hover:bg-orange-500 disabled:opacity-40 text-white text-sm font-semibold rounded-lg py-2 transition-colors"
          >
            {busy ? "Saving…" : "Save"}
          </button>
          <button
            onClick={onClose}
            className="flex-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-sm rounded-lg py-2 border border-zinc-700 transition-colors"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
