import { useRef } from "react";
import type { FormDataField, RequestBodyType } from "../types";
import { beautifyJson } from "../utils";
import Editor from "@monaco-editor/react";
import { cn } from "@/lib/utils";
import { useTheme } from "@/contexts/ThemeContext";
import { Braces, FileUp, Trash2, AlertCircle, CheckCircle2 } from "lucide-react";
import { BASE_EDITOR_OPTIONS, getEditorTheme } from "../editorConfig";
import { useMonacoShortcuts } from "../hooks/useMonacoShortcuts";

interface RequestBodyProps {
  bodyType: RequestBodyType;
  body: string;
  onBodyChange: (value: string) => void;
  onBodyTypeChange: (type: RequestBodyType) => void;
  formDataFields: FormDataField[];
  onFormDataChange: (fields: FormDataField[]) => void;
}

const BODY_TYPES: { value: RequestBodyType; label: string; desc: string }[] = [
  { value: "none", label: "None", desc: "No body" },
  { value: "json", label: "JSON", desc: "application/json" },
  { value: "text", label: "Text", desc: "text/plain" },
  { value: "form-data", label: "Form Data", desc: "multipart/form-data" },
];

function stripJsonComments(s: string): string {
  return s
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/\/\/[^\n\r]*/g, "")
    .trim();
}

function JsonStatus({ json }: { json: string }) {
  if (!json.trim()) return null;
  const isValid = (() => {
    try { JSON.parse(json); return true; } catch { /* fall through */ }
    try { JSON.parse(stripJsonComments(json)); return true; } catch { /* fall through */ }
    return false;
  })();
  if (isValid) {
    return (
      <span className="flex items-center gap-1 text-[10px] text-emerald-500 dark:text-emerald-400">
        <CheckCircle2 className="h-3 w-3" />Valid JSON
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1 text-[10px] text-red-500 dark:text-red-400">
      <AlertCircle className="h-3 w-3" />Invalid JSON
    </span>
  );
}

export function RequestBody({ bodyType, body, onBodyChange, onBodyTypeChange, formDataFields, onFormDataChange }: RequestBodyProps) {
  const { resolved } = useTheme();
  const fileInputRefs = useRef<Record<string, HTMLInputElement | null>>({});
  const onMount = useMonacoShortcuts();

  const addFormField = () => {
    onFormDataChange([...formDataFields, { id: crypto.randomUUID(), key: "", value: "", type: "text", enabled: true }]);
  };

  const removeFormField = (id: string) => {
    onFormDataChange(formDataFields.filter((f) => f.id !== id));
  };

  const updateFormField = (id: string, patch: Partial<FormDataField>) => {
    onFormDataChange(formDataFields.map((f) => f.id === id ? { ...f, ...patch } : f));
  };

  const handleTypeChange = (t: RequestBodyType) => {
    onBodyTypeChange(t);

    if (t === "json" && !body.trim()) {
      onBodyChange("{\n  \n}");
    }
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-1 px-3 py-1.5 border-b border-stone-200 dark:border-zinc-800 shrink-0">
        {BODY_TYPES.map((bt) => (
          <button
            key={bt.value}
            onClick={() => handleTypeChange(bt.value)}
            title={bt.desc}
            className={cn(
              "px-3 py-1 rounded-md text-xs font-medium transition-colors",
              bodyType === bt.value
                ? "bg-brand-600/20 text-brand-400 border border-brand-500/30"
                : "text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800",
            )}
          >
            {bt.label}
          </button>
        ))}

        {bodyType === "json" && (
          <div className="ml-auto flex items-center gap-2">
            <JsonStatus json={body} />
            <button
              onClick={() => { const b = beautifyJson(body); if (b) onBodyChange(b); }}
              className="flex items-center gap-1 px-2 py-0.5 text-xs text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800 rounded transition-colors"
              title="Beautify JSON"
            >
              <Braces className="h-3.5 w-3.5" />
              Beautify
            </button>
          </div>
        )}
      </div>

      <div className="flex-1 overflow-hidden">
        {bodyType === "none" && (
          <div className="flex flex-col items-center justify-center h-full gap-2 text-zinc-400 dark:text-zinc-700">
            <p className="text-xs">This request has no body.</p>
            <div className="flex gap-2">
              {(["json", "text", "form-data"] as RequestBodyType[]).map((t) => (
                <button
                  key={t}
                  onClick={() => handleTypeChange(t)}
                  className="px-3 py-1 text-xs bg-white dark:bg-zinc-800 hover:bg-stone-100 dark:hover:bg-zinc-700 text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 rounded-md border border-stone-300 dark:border-zinc-700 transition-colors capitalize"
                >
                  {t}
                </button>
              ))}
            </div>
          </div>
        )}

        {(bodyType === "json" || bodyType === "text") && (
          <div className="h-full" onMouseDown={(e) => e.stopPropagation()}>
            <Editor
              height="100%"
              language={bodyType === "json" ? "json" : "plaintext"}
              value={body}
              onChange={(v) => onBodyChange(v ?? "")}
              theme={getEditorTheme(resolved)}
              onMount={onMount}
              options={{
                ...BASE_EDITOR_OPTIONS,
                bracketPairColorization: { enabled: true },
                formatOnPaste: bodyType === "json",
                formatOnType: bodyType === "json",
                renderWhitespace: "selection" as const,
              }}
            />
          </div>
        )}

        {bodyType === "form-data" && (
          <div className="h-full flex flex-col overflow-hidden">
            <div className="flex-1 overflow-y-auto">
              {formDataFields.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-10 text-zinc-400 dark:text-zinc-700">
                  <p className="text-xs">No fields. Add key-value pairs or file uploads.</p>
                </div>
              ) : (
                <>
                  <div className="grid grid-cols-[20px_80px_1fr_1fr_28px] gap-1 px-3 py-1 border-b border-stone-200/60 dark:border-zinc-800/60">
                    <div />
                    <div className="text-[10px] uppercase tracking-wider text-zinc-400 dark:text-zinc-600 font-medium">Type</div>
                    <div className="text-[10px] uppercase tracking-wider text-zinc-400 dark:text-zinc-600 font-medium">Key</div>
                    <div className="text-[10px] uppercase tracking-wider text-zinc-400 dark:text-zinc-600 font-medium">Value / File</div>
                    <div />
                  </div>

                  {formDataFields.map((f) => (
                    <div
                      key={f.id}
                      className={cn(
                        "grid grid-cols-[20px_80px_1fr_1fr_28px] gap-1 px-3 py-1.5 items-center",
                        "border-b border-stone-200/30 dark:border-zinc-800/30 hover:bg-stone-100/40 dark:hover:bg-zinc-800/40 group",
                        !f.enabled && "opacity-40",
                      )}
                    >
                      <input
                        type="checkbox"
                        checked={f.enabled}
                        onChange={(e) => updateFormField(f.id, { enabled: e.target.checked })}
                        className="w-3.5 h-3.5 accent-brand-500 cursor-pointer"
                      />

                      <button
                        onClick={() => updateFormField(f.id, { type: f.type === "text" ? "file" : "text", value: "" })}
                        className={cn(
                          "px-2 py-0.5 text-[10px] rounded border font-medium transition-colors",
                          f.type === "file"
                            ? "text-purple-500 dark:text-purple-400 border-purple-500/30 bg-purple-500/10"
                            : "text-zinc-500 dark:text-zinc-500 border-stone-300 dark:border-zinc-700 hover:text-zinc-800 dark:hover:text-zinc-300",
                        )}
                      >
                        {f.type === "file" ? "FILE" : "TEXT"}
                      </button>

                      <input
                        value={f.key}
                        onChange={(e) => updateFormField(f.id, { key: e.target.value })}
                        placeholder="Key"
                        disabled={!f.enabled}
                        autoCorrect="off"
                        autoCapitalize="none"
                        spellCheck={false}
                        className="w-full bg-transparent border-b border-transparent hover:border-stone-300 dark:hover:border-zinc-700 focus:border-brand-500/60 px-1 py-0.5 text-xs font-medium text-zinc-700 dark:text-zinc-300 placeholder:text-zinc-500 dark:placeholder:text-zinc-700 focus:outline-none transition-colors"
                      />

                      {f.type === "text" ? (
                        <input
                          value={f.value}
                          onChange={(e) => updateFormField(f.id, { value: e.target.value })}
                          placeholder="Value"
                          disabled={!f.enabled}
                          autoCorrect="off"
                          autoCapitalize="none"
                          spellCheck={false}
                          className="w-full bg-transparent border-b border-transparent hover:border-stone-300 dark:hover:border-zinc-700 focus:border-brand-500/60 px-1 py-0.5 text-xs font-mono text-zinc-500 dark:text-zinc-400 placeholder:text-zinc-500 dark:placeholder:text-zinc-700 focus:outline-none transition-colors"
                        />
                      ) : (
                        <div className="flex items-center gap-1.5">
                          <input
                            ref={(el) => { fileInputRefs.current[f.id] = el; }}
                            type="file"
                            className="hidden"
                            onChange={async (e) => {
                              const file = e.target.files?.[0];
                              if (!file) return;
                              const reader = new FileReader();
                              reader.onload = () => {
                                const dataUrl = reader.result as string;
                                const base64 = dataUrl.split(",")[1] ?? "";
                                updateFormField(f.id, {
                                  value: file.name,
                                  fileName: file.name,
                                  fileData: base64,
                                  mimeType: file.type || "application/octet-stream",
                                });
                              };
                              reader.readAsDataURL(file);
                            }}
                          />
                          <button
                            onClick={() => fileInputRefs.current[f.id]?.click()}
                            disabled={!f.enabled}
                            className="flex items-center gap-1 px-2 py-0.5 text-xs bg-stone-100 dark:bg-zinc-700 hover:bg-stone-200 dark:hover:bg-zinc-600 text-zinc-700 dark:text-zinc-300 rounded transition-colors border border-stone-300 dark:border-zinc-600"
                          >
                            <FileUp className="h-3 w-3" />
                            {f.fileName ?? "Choose file"}
                          </button>
                        </div>
                      )}

                      <button
                        onClick={() => removeFormField(f.id)}
                        className="opacity-0 group-hover:opacity-100 flex items-center justify-center text-zinc-400 dark:text-zinc-600 hover:text-red-400 transition-all"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </div>
                  ))}
                </>
              )}

              <div
                onClick={addFormField}
                className="px-3 py-2 cursor-pointer hover:bg-stone-100/40 dark:hover:bg-zinc-800/40 border-b border-stone-200/30 dark:border-zinc-800/30"
              >
                <span className="text-xs text-zinc-400 dark:text-zinc-700 italic ml-6">+ Add field</span>
              </div>
            </div>

            {formDataFields.filter((f) => f.enabled && f.key).length > 0 && (
              <div className="shrink-0 border-t border-stone-200 dark:border-zinc-800 px-3 py-1.5">
                <span className="text-[10px] text-zinc-400 dark:text-zinc-600">
                  {formDataFields.filter((f) => f.enabled && f.key).length} field(s) ·{" "}
                  {formDataFields.filter((f) => f.enabled && f.type === "file").length} file(s)
                </span>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
