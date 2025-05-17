import { useState } from "react";
import { RequestBodyType } from "../types";
import { beautifyJson } from "../utils";
import Editor from "@monaco-editor/react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { Braces } from "lucide-react";

interface RequestBodyProps {
  bodyType: RequestBodyType;
  body: string;
  onBodyChange: (value: string) => void;
  onBodyTypeChange: (type: RequestBodyType) => void;
}

export function RequestBody({
  bodyType,
  body,
  onBodyChange,
  onBodyTypeChange
}: RequestBodyProps) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm">Request Body</CardTitle>
        <div className="flex items-center gap-2">
          {bodyType === 'json' && (
            <Button 
              variant="outline" 
              size="sm" 
              onClick={() => onBodyChange(beautifyJson(body))}
            >
              <Braces className="mr-1 h-4 w-4"/>
              Beautify
            </Button>
          )}
          <Select 
            value={bodyType} 
            onValueChange={(val: RequestBodyType) => onBodyTypeChange(val)}
          >
            <SelectTrigger className="w-[120px]"> 
              <SelectValue placeholder="Body Type"/> 
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">None</SelectItem>
              <SelectItem value="json">JSON</SelectItem>
              <SelectItem value="text">Text</SelectItem>
              {/* <SelectItem value="form-data">Form-data</SelectItem> */}
            </SelectContent>
          </Select>
        </div>
      </CardHeader>
      <CardContent>
        {bodyType === 'json' && (
          <Editor 
            height="200px" 
            language="json" 
            value={body} 
            onChange={(val) => onBodyChange(val || "")} 
            theme="vs-dark" 
            options={{ minimap: { enabled: false } }}
          />
        )}
        {bodyType === 'text' && (
          <Textarea 
            placeholder="Plain text content" 
            className="h-40 font-mono" 
            value={body} 
            onChange={(e) => onBodyChange(e.target.value)} 
          />
        )}
        {bodyType === 'none' && (
          <p className="text-sm text-muted-foreground p-4 text-center">
            This request does not have a body.
          </p>
        )}
      </CardContent>
    </Card>
  );
}
