import { useState } from "react";
import Editor from "@monaco-editor/react";
import { cn } from "@/lib/utils";
import { Play, Info } from "lucide-react";

interface ScriptsTabProps {
  preRequestScript: string;
  onPreRequestScriptChange: (v: string) => void;
  testScript: string;
  onTestScriptChange: (v: string) => void;
}

const PRESET_SNIPPETS = {
  preRequest: [
    { label: "Log env variable", code: `// Access environment variable\nconsole.log(pm.environment.get("baseUrl"));\n` },
    { label: "Set random ID", code: `pm.environment.set("randomId", Math.random().toString(36).slice(2));\n` },
  ],
  tests: [
    { label: "Status 200", code: `pm.test("Status is 200", () => {\n  pm.response.to.have.status(200);\n});\n` },
    { label: "JSON body", code: `pm.test("Response is JSON", () => {\n  pm.response.to.be.json;\n});\n` },
    { label: "Response time", code: `pm.test("Response time < 500ms", () => {\n  pm.expect(pm.response.responseTime).to.be.below(500);\n});\n` },
    { label: "Contains key", code: `pm.test("Has 'id' key", () => {\n  const json = pm.response.json();\n  pm.expect(json).to.have.property("id");\n});\n` },
  ],
};

export function ScriptsTab({ preRequestScript, onPreRequestScriptChange, testScript, onTestScriptChange }: ScriptsTabProps) {
  const [activeScript, setActiveScript] = useState<"pre" | "test">("pre");

  const currentValue = activeScript === "pre" ? preRequestScript : testScript;
  const currentOnChange = activeScript === "pre" ? onPreRequestScriptChange : onTestScriptChange;
  const snippets = activeScript === "pre" ? PRESET_SNIPPETS.preRequest : PRESET_SNIPPETS.tests;

  return (
    <div className="flex flex-col h-full">
      {/* Sub-tab selector */}
      <div className="flex items-center gap-1 px-3 py-1.5 border-b border-zinc-800 shrink-0">
        {(["pre", "test"] as const).map((s) => (
          <button
            key={s}
            onClick={() => setActiveScript(s)}
            className={cn(
              "px-3 py-1 rounded-md text-xs font-medium transition-colors",
              activeScript === s
                ? "bg-orange-600/20 text-orange-400 border border-orange-500/30"
                : "text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800",
            )}
          >
            {s === "pre" ? "Pre-request" : "Tests"}
          </button>
        ))}

        {/* Snippets */}
        <div className="ml-auto flex items-center gap-1">
          {snippets.map((snip) => (
            <button
              key={snip.label}
              onClick={() => currentOnChange(currentValue ? `${currentValue}\n${snip.code}` : snip.code)}
              className="flex items-center gap-1 px-2 py-0.5 text-[10px] text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 rounded border border-zinc-800 transition-colors"
              title={`Insert: ${snip.label}`}
            >
              <Play className="h-2.5 w-2.5" />
              {snip.label}
            </button>
          ))}
        </div>
      </div>

      {/* Info bar */}
      <div className="flex items-center gap-2 px-3 py-1.5 bg-zinc-900/60 border-b border-zinc-800/50 shrink-0">
        <Info className="h-3 w-3 text-zinc-600 shrink-0" />
        <p className="text-[10px] text-zinc-600">
          {activeScript === "pre"
            ? "Runs before the request is sent. Use pm.environment.set() to inject variables."
            : "Runs after response is received. Use pm.test() and pm.expect() for assertions."}
        </p>
      </div>

      {/* Editor */}
      <div className="flex-1 overflow-hidden" onMouseDown={(e) => e.stopPropagation()}>
        <Editor
          height="100%"
          language="javascript"
          value={currentValue}
          onChange={(v) => currentOnChange(v ?? "")}
          theme="vs-dark"
          options={{
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            fontSize: 13,
            lineNumbers: "on",
            wordWrap: "on",
            folding: true,
            padding: { top: 8, bottom: 8 },
            tabSize: 2,
            suggest: { showKeywords: true },
          }}
        />
      </div>
    </div>
  );
}
