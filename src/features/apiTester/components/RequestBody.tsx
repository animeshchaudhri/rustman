import { useRef, useState } from "react";
import type { FormDataField, RequestBodyType } from "../types";
import { beautifyJson } from "../utils";
import Editor from "@monaco-editor/react";
import { cn } from "@/lib/utils";
import { Braces, FileUp, Plus, Trash2, AlertCircle, CheckCircle2 } from "lucide-react";

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

function JsonStatus({ json }: { json: string }) {
  if (!json.trim()) return null;
  try {
    JSON.parse(json);
    return (
      <span className="flex items-center gap-1 text-[10px] text-emerald-400">
        <CheckCircle2 className="h-3 w-3" />Valid JSON
      </span>
    );
  } catch (e: unknown) {
    const msg = e instanceof SyntaxError ? e.message : "Invalid JSON";
    return (
      <span className="flex items-center gap-1 text-[10px] text-red-400" title={msg}>
        <AlertCircle className="h-3 w-3" />Invalid JSON
      </span>
    );
  }
}

export function RequestBody({ bodyType, body, onBodyChange, onBodyTypeChange, formDataFields, onFormDataChange }: RequestBodyProps) {
  const fileInputRefs = useRef<Record<string, HTMLInputElement | null>>({});
  const [editorMounted, setEditorMounted] = useState(false);

  const addFormField = () => {
    onFormDataChange([...formDataFields, { id: crypto.randomUUID(), key: "", value: "", type: "text", enabled: true }]);
  };

  const removeFormField = (id: string) => {
    onFormDataChange(formDataFields.filter(f => f.id !== id));
  };

  const updateFormField = (id: string, patch: Partial<FormDataField>) => {
    onFormDataChange(formDataFields.map(f => f.id === id ? { ...f, ...patch } : f));
  };

  const handleTypeChange = (t: RequestBodyType) => {
    onBodyTypeChange(t);
    // Auto-set starter templates
    if (t === "json" && !body.trim()) {
      onBodyChange("{\n  \n}");
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Type selector */}
      <div className="flex items-center gap-1 px-3 py-1.5 border-b border-zinc-800 shrink-0">
        {BODY_TYPES.map(bt => (
          <button
            key={bt.value}
            onClick={() => handleTypeChange(bt.value)}
            title={bt.desc}
            className={cn(
              "px-3 py-1 rounded-md text-xs font-medium transition-colors",
              bodyType === bt.value
                ? "bg-orange-600/20 text-orange-400 border border-orange-500/30"
                : "text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800",
            )}
          >
            {bt.label}
          </button>
        ))}

        {/* JSON tools */}
        {bodyType === "json" && (
          <div className="ml-auto flex items-center gap-2">
            <JsonStatus json={body} />
            <button
              onClick={() => { const b = beautifyJson(body); if (b) onBodyChange(b); }}
              className="flex items-center gap-1 px-2 py-0.5 text-xs text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 rounded transition-colors"
              title="Beautify JSON"
            >
              <Braces className="h-3.5 w-3.5" />
              Beautify
            </button>
          </div>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        {bodyType === "none" && (
          <div className="flex flex-col items-center justify-center h-full gap-2 text-zinc-700">
            <p className="text-xs">This request has no body.</p>
            <div className="flex gap-2">
              {(["json", "text", "form-data"] as RequestBodyType[]).map(t => (
                <button
                  key={t}
                  onClick={() => handleTypeChange(t)}
                  className="px-3 py-1 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 rounded-md border border-zinc-700 transition-colors capitalize"
                >
                  {t}
                </button>
              ))}
            </div>
          </div>
        )}

        {(bodyType === "json" || bodyType === "text") && (
          <div className="h-full" onMouseDown={e => e.stopPropagation()}>
            <Editor
              height="100%"
              language={bodyType === "json" ? "json" : "plaintext"}
              value={body}
              onChange={v => onBodyChange(v ?? "")}
              onMount={() => setEditorMounted(true)}
              theme="vs-dark"
              options={{
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                fontSize: 13,
                lineNumbers: "on",
                wordWrap: "on",
                folding: true,
                bracketPairColorization: { enabled: true },
                padding: { top: 8, bottom: 8 },
                suggest: { showKeywords: true },
                formatOnPaste: bodyType === "json",
                formatOnType: bodyType === "json",
                renderWhitespace: "selection",
                tabSize: 2,
              }}
            />
          </div>
        )}

        {bodyType === "form-data" && (
          <div className="h-full flex flex-col overflow-hidden">
            <div className="flex-1 overflow-y-auto">
              {formDataFields.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-10 text-zinc-700">
                  <p className="text-xs">No fields. Add key-value pairs or file uploads.</p>
                </div>
              ) : (
                <>
                  {/* Column headings */}
                  <div className="grid grid-cols-[20px_80px_1fr_1fr_28px] gap-1 px-3 py-1 border-b border-zinc-800/60">
                    <div />
                    <div className="text-[10px] uppercase tracking-wider text-zinc-600 font-medium">Type</div>
                    <div className="text-[10px] uppercase tracking-wider text-zinc-600 font-medium">Key</div>
                    <div className="text-[10px] uppercase tracking-wider text-zinc-600 font-medium">Value / File</div>
                    <div />
                  </div>

                  {formDataFields.map(f => (
                    <div
                      key={f.id}
                      className={cn(
                        "grid grid-cols-[20px_80px_1fr_1fr_28px] gap-1 px-3 py-1.5 items-center",
                        "border-b border-zinc-800/30 hover:bg-zinc-800/40 group",
                        !f.enabled && "opacity-40",
                      )}
                    >
                      <input
                        type="checkbox"
                        checked={f.enabled}
                        onChange={e => updateFormField(f.id, { enabled: e.target.checked })}
                        className="w-3.5 h-3.5 accent-orange-500 cursor-pointer"
                      />

                      {/* Type toggle */}
                      <button
                        onClick={() => updateFormField(f.id, { type: f.type === "text" ? "file" : "text", value: "" })}
                        className={cn(
                          "px-2 py-0.5 text-[10px] rounded border font-medium transition-colors",
                          f.type === "file"
                            ? "text-purple-400 border-purple-500/30 bg-purple-500/10"
                            : "text-zinc-500 border-zinc-700 hover:text-zinc-300",
                        )}
                      >
                        {f.type === "file" ? "FILE" : "TEXT"}
                      </button>

                      <input
                        value={f.key}
                        onChange={e => updateFormField(f.id, { key: e.target.value })}
                        placeholder="Key"
                        disabled={!f.enabled}
                        className="w-full bg-transparent border-b border-transparent hover:border-zinc-700 focus:border-orange-500/60 px-1 py-0.5 text-xs font-medium text-zinc-300 placeholder:text-zinc-700 focus:outline-none transition-colors"
                      />

                      {f.type === "text" ? (
                        <input
                          value={f.value}
                          onChange={e => updateFormField(f.id, { value: e.target.value })}
                          placeholder="Value"
                          disabled={!f.enabled}
                          className="w-full bg-transparent border-b border-transparent hover:border-zinc-700 focus:border-orange-500/60 px-1 py-0.5 text-xs font-mono text-zinc-400 placeholder:text-zinc-700 focus:outline-none transition-colors"
                        />
                      ) : (
                        <div className="flex items-center gap-1.5">
                          <input
                            ref={el => { fileInputRefs.current[f.id] = el; }}
                            type="file"
                            className="hidden"
                            onChange={e => {
                              const file = e.target.files?.[0];
                              if (file) updateFormField(f.id, { value: file.name, fileName: file.name });
                            }}
                          />
                          <button
                            onClick={() => fileInputRefs.current[f.id]?.click()}
                            disabled={!f.enabled}
                            className="flex items-center gap-1 px-2 py-0.5 text-xs bg-zinc-700 hover:bg-zinc-600 text-zinc-300 rounded transition-colors border border-zinc-600"
                          >
                            <FileUp className="h-3 w-3" />
                            {f.fileName ?? "Choose file"}
                          </button>
                        </div>
                      )}

                      <button
                        onClick={() => removeFormField(f.id)}
                        className="opacity-0 group-hover:opacity-100 flex items-center justify-center text-zinc-600 hover:text-red-400 transition-all"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </div>
                  ))}
                </>
              )}

              <div
                onClick={addFormField}
                className="px-3 py-2 cursor-pointer hover:bg-zinc-800/40 border-b border-zinc-800/30"
              >
                <span className="text-xs text-zinc-700 italic ml-6">+ Add field</span>
              </div>
            </div>

            {/* Summary */}
            {formDataFields.filter(f => f.enabled && f.key).length > 0 && (
              <div className="shrink-0 border-t border-zinc-800 px-3 py-1.5">
                <span className="text-[10px] text-zinc-600">
                  {formDataFields.filter(f => f.enabled && f.key).length} field(s) ·{" "}
                  {formDataFields.filter(f => f.enabled && f.type === "file").length} file(s)
                </span>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
