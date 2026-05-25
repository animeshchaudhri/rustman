import { useState } from "react";
import Editor from "@monaco-editor/react";
import { cn } from "@/lib/utils";
import { useTheme } from "@/contexts/ThemeContext";
import { Play, Info } from "lucide-react";
import { BASE_EDITOR_OPTIONS, getEditorTheme } from "../editorConfig";
import { useMonacoShortcuts } from "../hooks/useMonacoShortcuts";

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
    {
      label: "AES-256-CBC encrypt body", code: `const crypto = require("crypto");

const rawBody = req.getBody();
const aesKey = pm.environment.get("aesKey"); // set in Environments

const bodyString = typeof rawBody === "string" ? rawBody : JSON.stringify(rawBody);

const iv = crypto.randomBytes(16);
const ivBase64 = iv.toString("base64");

const cipher = crypto.createCipheriv("aes-256-cbc", Buffer.from(aesKey, "utf8"), iv);
let encrypted = cipher.update(bodyString, "utf8", "base64");
encrypted += await cipher.final("base64");

const finalEncryptedRequest = ivBase64 + "." + encrypted;
req.setBody(JSON.stringify({ EncryptedPayload: finalEncryptedRequest }));
console.log("Encrypted:", finalEncryptedRequest);
`,
    },
  ],
  tests: [
    { label: "Status 200", code: `pm.test("Status is 200", () => {\n  pm.response.to.have.status(200);\n});\n` },
    { label: "JSON body", code: `pm.test("Response is JSON", () => {\n  pm.response.to.be.json;\n});\n` },
    { label: "Response time", code: `pm.test("Response time < 500ms", () => {\n  pm.expect(pm.response.responseTime).to.be.below(500);\n});\n` },
    { label: "Contains key", code: `pm.test("Has 'id' key", () => {\n  const json = pm.response.json();\n  pm.expect(json).to.have.property("id");\n});\n` },
    {
      label: "AES-256-CBC decrypt response", code: `const crypto = require("crypto");

const body = res.json();
const aesKey = pm.environment.get("aesKey"); // set in Environments

const [ivBase64, encryptedData] = body.EncryptedPayload.split(".");
const iv = Buffer.from(ivBase64, "base64");

const decipher = crypto.createDecipheriv("aes-256-cbc", Buffer.from(aesKey, "utf8"), iv);
decipher.update(encryptedData, "base64");
const decrypted = await decipher.final("utf8");

console.log("Decrypted:", decrypted);
pm.environment.set("decryptedResponse", decrypted);
`,
    },
    {
      label: "HMAC-SHA256 verify", code: `const crypto = require("crypto");

const secret = pm.environment.get("hmacSecret");
const body = res.text();

const hmac = crypto.createHmac("sha256", Buffer.from(secret, "utf8"));
hmac.update(body);
const sig = await hmac.digest("hex");

pm.test("HMAC signature matches", () => {
  const expected = res.getHeader("x-signature");
  pm.expect(sig).to.equal(expected);
});
console.log("Computed HMAC:", sig);
`,
    },
  ],
};

export function ScriptsTab({ preRequestScript, onPreRequestScriptChange, testScript, onTestScriptChange }: ScriptsTabProps) {
  const { resolved } = useTheme();
  const [activeScript, setActiveScript] = useState<"pre" | "test">("pre");
  const onMount = useMonacoShortcuts();

  const currentValue = activeScript === "pre" ? preRequestScript : testScript;
  const currentOnChange = activeScript === "pre" ? onPreRequestScriptChange : onTestScriptChange;
  const snippets = activeScript === "pre" ? PRESET_SNIPPETS.preRequest : PRESET_SNIPPETS.tests;

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-1 px-3 py-1.5 border-b border-stone-200 dark:border-zinc-800 shrink-0">
        {(["pre", "test"] as const).map((s) => (
          <button
            key={s}
            onClick={() => setActiveScript(s)}
            className={cn(
              "px-3 py-1 rounded-md text-xs font-medium transition-colors",
              activeScript === s
                ? "bg-brand-600/20 text-brand-400 border border-brand-500/30"
                : "text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800",
            )}
          >
            {s === "pre" ? "Pre-request" : "Tests"}
          </button>
        ))}

        <div className="ml-auto flex items-center gap-1">
          {snippets.map((snip) => (
            <button
              key={snip.label}
              onClick={() => currentOnChange(currentValue ? `${currentValue}\n${snip.code}` : snip.code)}
              className="flex items-center gap-1 px-2 py-0.5 text-[10px] text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800 rounded border border-stone-200 dark:border-zinc-800 transition-colors"
              title={`Insert: ${snip.label}`}
            >
              <Play className="h-2.5 w-2.5" />
              {snip.label}
            </button>
          ))}
        </div>
      </div>

      <div className="flex items-center gap-2 px-3 py-1.5 bg-stone-50/60 dark:bg-zinc-900/60 border-b border-stone-200/50 dark:border-zinc-800/50 shrink-0">
        <Info className="h-3 w-3 text-zinc-400 dark:text-zinc-600 shrink-0" />
        <p className="text-[10px] text-zinc-400 dark:text-zinc-600">
          {activeScript === "pre"
            ? "Runs before the request is sent. Globals: pm, req, require('crypto'), Buffer."
            : "Runs after response is received. Globals: pm, res, require('crypto'), Buffer."}
        </p>
      </div>

      <div className="flex-1 overflow-hidden" onMouseDown={(e) => e.stopPropagation()}>
        <Editor
          height="100%"
          language="javascript"
          value={currentValue}
          onChange={(v) => currentOnChange(v ?? "")}
          theme={getEditorTheme(resolved)}
          onMount={onMount}
          options={BASE_EDITOR_OPTIONS}
        />
      </div>
    </div>
  );
}
