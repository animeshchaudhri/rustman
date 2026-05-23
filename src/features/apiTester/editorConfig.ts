import type * as Monaco from "monaco-editor";

export function getEditorTheme(resolved: "dark" | "light"): string {
  return resolved === "dark" ? "vs-dark" : "vs";
}

export const BASE_EDITOR_OPTIONS: Monaco.editor.IStandaloneEditorConstructionOptions = {
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
  fontSize: 13,
  lineNumbers: "on" as const,
  wordWrap: "on" as const,
  folding: true,
  padding: { top: 8, bottom: 8 },
  tabSize: 2,
  suggest: { showKeywords: true },
};

export const READ_ONLY_EDITOR_OPTIONS: Monaco.editor.IStandaloneEditorConstructionOptions = {
  ...BASE_EDITOR_OPTIONS,
  readOnly: true,
  lineNumbers: "off" as const,
  renderLineHighlight: "none" as const,
  padding: { top: 16, bottom: 16 },
};
