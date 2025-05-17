import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { Code, Copy, DownloadCloud } from "lucide-react";
import Editor from "@monaco-editor/react";

interface CodeGenerationProps {
  onGenerateCurl: () => void;
  onPrepareJsCode: () => void;
  generatedCurl: string;
  generatedJs: string;
  onCurlImport: (curl: string) => void;
}

export function CodeGeneration({
  onGenerateCurl,
  onPrepareJsCode,
  generatedCurl,
  generatedJs,
  onCurlImport
}: CodeGenerationProps) {
  const [curlImportInput, setCurlImportInput] = useState<string>("");

  const handleGenerateCode = () => {
    onGenerateCurl();
    onPrepareJsCode();
  };

  const handleImport = () => {
    onCurlImport(curlImportInput);
    setCurlImportInput("");
    // Close the dialog
    const closeButton = document.querySelector('[data-radix-dialog-default-trigger="true"]');
    if (closeButton instanceof HTMLElement) closeButton.click();
  };

  return (
    <div className="flex items-center gap-2 mb-4">
      <Dialog>
        <DialogTrigger asChild>
          <Button variant="outline" size="sm">
            <DownloadCloud className="mr-2 h-4 w-4"/> Import cURL
          </Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Import cURL Command</DialogTitle>
          </DialogHeader>
          <Textarea 
            placeholder="Paste cURL command here..."
            value={curlImportInput}
            onChange={e => setCurlImportInput(e.target.value)}
            className="h-40 min-h-[100px] font-mono text-xs"
          />
          <DialogFooter>
            <Button onClick={handleImport}>Import</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      
      <Dialog>
        <DialogTrigger asChild>
          <Button variant="outline" size="sm" onClick={handleGenerateCode}>
            <Code className="mr-2 h-4 w-4"/> Generate Code
          </Button>
        </DialogTrigger>
        <DialogContent className="max-w-[70vw]">
          <DialogHeader>
            <DialogTitle>Generated Code</DialogTitle>
          </DialogHeader>
          <Tabs defaultValue="curl">
            <TabsList>
              <TabsTrigger value="curl">cURL</TabsTrigger>
              <TabsTrigger value="javascript">JavaScript (fetch)</TabsTrigger>
            </TabsList>
            <TabsContent value="curl">
              <div className="relative mt-2">
                <Editor 
                  height="40vh" 
                  language="shell" 
                  value={generatedCurl} 
                  theme="vs-dark" 
                  options={{readOnly: true, minimap: {enabled: false}}}
                />
                <Button 
                  size="icon" 
                  variant="ghost" 
                  className="absolute top-2 right-2" 
                  onClick={() => navigator.clipboard.writeText(generatedCurl)}
                >
                  <Copy className="h-4 w-4"/>
                </Button>
              </div>
            </TabsContent>
            <TabsContent value="javascript">
              <div className="relative mt-2">
                <Editor 
                  height="40vh" 
                  language="javascript" 
                  value={generatedJs} 
                  theme="vs-dark" 
                  options={{readOnly: true, minimap: {enabled: false}}}
                />
                <Button 
                  size="icon" 
                  variant="ghost" 
                  className="absolute top-2 right-2" 
                  onClick={() => navigator.clipboard.writeText(generatedJs)}
                >
                  <Copy className="h-4 w-4"/>
                </Button>
              </div>
            </TabsContent>
          </Tabs>
        </DialogContent>
      </Dialog>
    </div>
  );
}
