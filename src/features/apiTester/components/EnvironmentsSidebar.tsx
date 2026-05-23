import { useState } from "react";
import type { AppEnvironment } from "../types";
import { cn } from "@/lib/utils";
import { Check, Globe, Pencil, Plus, Trash2 } from "lucide-react";

interface EnvEditorState {
  id: string;
  name: string;
  variables: Array<{ key: string; value: string }>;
}

interface EnvironmentsSidebarProps {
  environments: AppEnvironment[];
  activeEnvId: string | null;
  envEnabled: boolean;
  onEnvEnabledChange: (enabled: boolean) => void;
  onSetActive: (id: string | null) => void;
  onSave: (env: AppEnvironment) => void;
  onDelete: (id: string) => void;
}

export function EnvironmentsSidebar({
  environments,
  activeEnvId,
  envEnabled,
  onEnvEnabledChange,
  onSetActive,
  onSave,
  onDelete,
}: EnvironmentsSidebarProps) {
  const [editor, setEditor] = useState<EnvEditorState | null>(null);

  const openEditor = (env?: AppEnvironment) => {
    if (env) {
      setEditor({
        id: env.id,
        name: env.name,
        variables: Object.entries(env.variables).map(([key, value]) => ({ key, value })),
      });
    } else {
      setEditor({ id: crypto.randomUUID(), name: "", variables: [{ key: "baseUrl", value: "" }] });
    }
  };

  const saveEditor = () => {
    if (!editor || !editor.name.trim()) return;
    const variables: Record<string, string> = {};
    for (const { key, value } of editor.variables) {
      if (key.trim()) variables[key.trim()] = value;
    }
    onSave({
      id: editor.id,
      name: editor.name.trim(),
      variables,
      isActive: activeEnvId === editor.id,
    });
    setEditor(null);
  };

  const updateEditorVar = (idx: number, field: "key" | "value", val: string) => {
    setEditor((prev) =>
      prev
        ? {
            ...prev,
            variables: prev.variables.map((v, i) => (i === idx ? { ...v, [field]: val } : v)),
          }
        : null,
    );
  };

  const addVar = () => {
    setEditor((prev) =>
      prev ? { ...prev, variables: [...prev.variables, { key: "", value: "" }] } : null,
    );
  };

  const removeVar = (idx: number) => {
    setEditor((prev) =>
      prev
        ? { ...prev, variables: prev.variables.filter((_, i) => i !== idx) }
        : null,
    );
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-3 py-2.5 border-b border-stone-300/50 dark:border-zinc-700/50">
        <span className="text-xs font-semibold text-zinc-500 dark:text-zinc-400 uppercase tracking-wider">
          Environments
        </span>
        {!editor && (
          <button
            onClick={() => openEditor()}
            className="p-1 text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-200 dark:hover:bg-zinc-700 rounded transition-colors"
            title="New environment"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        )}
      </div>

      <div className="flex items-center justify-between px-3 py-2 border-b border-stone-300/30 dark:border-zinc-700/30">
        <span className="text-xs text-zinc-500 dark:text-zinc-400">Environment mode</span>
        <button
          onClick={() => onEnvEnabledChange(!envEnabled)}
          className={cn(
            "relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none",
            envEnabled ? "bg-brand-600" : "bg-stone-300 dark:bg-zinc-600",
          )}
        >
          <span
            className={cn(
              "inline-block h-3.5 w-3.5 rounded-full bg-white shadow transition-transform",
              envEnabled ? "translate-x-5" : "translate-x-1",
            )}
          />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto py-1">
        {editor ? (
          <div className="p-3 space-y-3">
            <input
              autoFocus
              value={editor.name}
              onChange={(e) => setEditor({ ...editor, name: e.target.value })}
              placeholder="Environment name"
              autoCorrect="off"
              autoCapitalize="none"
              spellCheck={false}
              className="w-full bg-white dark:bg-zinc-800 border border-stone-300 dark:border-zinc-700 rounded-md px-2.5 py-1.5 text-sm text-zinc-800 dark:text-zinc-200 focus:outline-none focus:border-brand-500/60 placeholder:text-zinc-400 dark:placeholder:text-zinc-600"
            />

            <div className="space-y-1">
              <div className="grid grid-cols-2 gap-1 text-[10px] text-zinc-500 dark:text-zinc-500 font-medium uppercase tracking-wide px-1">
                <span>Variable</span>
                <span>Value</span>
              </div>
              {editor.variables.map((v, i) => (
                <div key={i} className="flex items-center gap-1">
                  <input
                    value={v.key}
                    onChange={(e) => updateEditorVar(i, "key", e.target.value)}
                    placeholder="key"
                    autoCorrect="off"
                    autoCapitalize="none"
                    spellCheck={false}
                    className="flex-1 bg-white dark:bg-zinc-800 border border-stone-300 dark:border-zinc-700 rounded px-2 py-1 text-xs text-zinc-700 dark:text-zinc-300 focus:outline-none focus:border-brand-500/50 placeholder:text-zinc-400 dark:placeholder:text-zinc-600"
                  />
                  <input
                    value={v.value}
                    onChange={(e) => updateEditorVar(i, "value", e.target.value)}
                    placeholder="value"
                    autoCorrect="off"
                    autoCapitalize="none"
                    spellCheck={false}
                    className="flex-1 bg-white dark:bg-zinc-800 border border-stone-300 dark:border-zinc-700 rounded px-2 py-1 text-xs text-zinc-700 dark:text-zinc-300 focus:outline-none focus:border-brand-500/50 placeholder:text-zinc-400 dark:placeholder:text-zinc-600"
                  />
                  <button
                    onClick={() => removeVar(i)}
                    className="text-zinc-400 dark:text-zinc-600 hover:text-red-400 p-0.5"
                  >
                    <Trash2 className="h-3 w-3" />
                  </button>
                </div>
              ))}
              <button
                onClick={addVar}
                className="text-xs text-zinc-500 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 flex items-center gap-1 mt-1"
              >
                <Plus className="h-3 w-3" />
                Add variable
              </button>
            </div>

            <div className="flex gap-2 pt-1">
              <button
                onClick={saveEditor}
                className="flex-1 bg-brand-600 hover:bg-brand-500 text-white text-xs font-medium rounded-md py-1.5 transition-colors"
              >
                Save
              </button>
              <button
                onClick={() => setEditor(null)}
                className="flex-1 bg-stone-200 dark:bg-zinc-700 hover:bg-stone-300 dark:hover:bg-zinc-600 text-zinc-700 dark:text-zinc-300 text-xs font-medium rounded-md py-1.5 transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <>
            {environments.length === 0 && (
              <div className="flex flex-col items-center justify-center py-12 px-4 text-center">
                <Globe className="h-8 w-8 text-zinc-400 dark:text-zinc-600 mb-2" />
                <p className="text-xs text-zinc-500 dark:text-zinc-500">No environments</p>
                {!envEnabled && (
                  <p className="text-xs text-zinc-400 dark:text-zinc-600 mt-1">Enable environment mode above</p>
                )}
              </div>
            )}

            <button
              className={cn(
                "w-full flex items-center gap-2 px-3 py-2 text-xs text-left hover:bg-stone-100 dark:hover:bg-zinc-800 transition-colors",
                activeEnvId === null && "bg-stone-100 dark:bg-zinc-800",
              )}
              onClick={() => onSetActive(null)}
            >
              <span className="flex-1 text-zinc-500 dark:text-zinc-400">No Environment</span>
              {activeEnvId === null && envEnabled && (
                <Check className="h-3.5 w-3.5 text-brand-400 shrink-0" />
              )}
            </button>

            {environments.map((env) => (
              <div
                key={env.id}
                className={cn(
                  "group flex items-center gap-2 px-3 py-2 hover:bg-stone-100 dark:hover:bg-zinc-800 cursor-pointer transition-colors",
                  activeEnvId === env.id && envEnabled && "bg-stone-100 dark:bg-zinc-800",
                )}
                onClick={() => onSetActive(env.id)}
              >
                <div className="flex-1 min-w-0">
                  <p className="text-xs text-zinc-700 dark:text-zinc-300 truncate font-medium">{env.name}</p>
                  {env.variables.baseUrl && (
                    <p className="text-[10px] text-zinc-400 dark:text-zinc-600 truncate font-mono">
                      {env.variables.baseUrl}
                    </p>
                  )}
                </div>

                {activeEnvId === env.id && envEnabled && (
                  <Check className="h-3.5 w-3.5 text-brand-400 shrink-0" />
                )}

                <div className="flex gap-0.5 opacity-0 group-hover:opacity-100 shrink-0">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      openEditor(env);
                    }}
                    className="p-1 text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-200 dark:hover:bg-zinc-700 rounded transition-colors"
                  >
                    <Pencil className="h-3 w-3" />
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onDelete(env.id);
                      if (activeEnvId === env.id) onSetActive(null);
                    }}
                    className="p-1 text-zinc-500 dark:text-zinc-500 hover:text-red-400 hover:bg-stone-200 dark:hover:bg-zinc-700 rounded transition-colors"
                  >
                    <Trash2 className="h-3 w-3" />
                  </button>
                </div>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}
