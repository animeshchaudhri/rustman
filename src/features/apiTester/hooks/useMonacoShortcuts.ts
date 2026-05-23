import { useCallback } from "react";
import type * as MonacoType from "monaco-editor";

export type AppShortcut = "new-tab" | "close-tab" | "send" | "save" | "duplicate" | "palette";

function dispatch(key: AppShortcut) {
  window.dispatchEvent(new CustomEvent("app:shortcut", { detail: key }));
}

export function useMonacoShortcuts() {
  return useCallback((editor: MonacoType.editor.IStandaloneCodeEditor, monaco: typeof MonacoType) => {
    const cmd = monaco.KeyMod.CtrlCmd;
    editor.addCommand(cmd | monaco.KeyCode.KeyT, () => dispatch("new-tab"));
    editor.addCommand(cmd | monaco.KeyCode.KeyW, () => dispatch("close-tab"));
    editor.addCommand(cmd | monaco.KeyCode.Enter, () => dispatch("send"));
    editor.addCommand(cmd | monaco.KeyCode.KeyS, () => dispatch("save"));
    editor.addCommand(cmd | monaco.KeyCode.KeyP, () => dispatch("palette"));
    editor.addCommand(cmd | monaco.KeyCode.KeyZ, () => editor.trigger("keyboard", "undo", null));
    editor.addCommand(cmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyZ, () => editor.trigger("keyboard", "redo", null));
    editor.addCommand(cmd | monaco.KeyCode.KeyY, () => editor.trigger("keyboard", "redo", null));
  }, []);
}
