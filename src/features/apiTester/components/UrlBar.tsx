import { Environment } from "../types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Send } from "lucide-react";

interface UrlBarProps {
  method: string;
  onMethodChange: (method: string) => void;
  urlInput: string;
  onUrlChange: (url: string) => void;
  activeEnvName: string;
  onEnvChange: (env: string) => void;
  environments: Environment[];
  isLoading: boolean;
  onSendRequest: () => void;
}

export function UrlBar({
  method,
  onMethodChange,
  urlInput,
  onUrlChange,
  activeEnvName,
  onEnvChange,
  environments,
  isLoading,
  onSendRequest
}: UrlBarProps) {
  return (
    <div className="flex items-center gap-2 mb-4">
      <Select value={method} onValueChange={onMethodChange}>
        <SelectTrigger className="w-[100px]">
          <SelectValue placeholder="Method" />
        </SelectTrigger>
        <SelectContent>
          {["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"].map(m => (
            <SelectItem key={m} value={m}>{m}</SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Input
        className="flex-1"
        placeholder="Enter request URL or endpoint"
        value={urlInput}
        onChange={(e) => onUrlChange(e.target.value)}
      />
      <Select value={activeEnvName} onValueChange={onEnvChange}>
        <SelectTrigger className="w-[150px]">
          <SelectValue placeholder="Environment" />
        </SelectTrigger>
        <SelectContent>
          {environments.map(env => (
            <SelectItem key={env.name} value={env.name}>{env.name}</SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Button onClick={onSendRequest} disabled={isLoading}>
        {isLoading ? "Sending..." : <Send className="h-4 w-4" />}
      </Button>
    </div>
  );
}
