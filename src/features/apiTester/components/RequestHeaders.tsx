import { HeaderType } from "../types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Trash2 } from "lucide-react";

interface RequestHeadersProps {
  headers: HeaderType[];
  onAddHeader: () => void;
  onHeaderChange: (id: string, field: "key" | "value" | "enabled", value: string | boolean) => void;
  onRemoveHeader: (id: string) => void;
}

export function RequestHeaders({ 
  headers, 
  onAddHeader, 
  onHeaderChange, 
  onRemoveHeader 
}: RequestHeadersProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm">Headers</CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {headers.map((header) => (
          <div key={header.id} className="flex gap-2 items-center">
            <Input 
              type="checkbox" 
              checked={header.enabled} 
              onChange={(e) => onHeaderChange(header.id, "enabled", e.target.checked)} 
              className="w-5 h-5"
            />
            <Input 
              placeholder="Key" 
              value={header.key} 
              onChange={(e) => onHeaderChange(header.id, "key", e.target.value)} 
              disabled={!header.enabled}
            />
            <Input 
              placeholder="Value" 
              value={header.value} 
              onChange={(e) => onHeaderChange(header.id, "value", e.target.value)} 
              disabled={!header.enabled}
            />
            <Button 
              variant="ghost" 
              size="icon" 
              onClick={() => onRemoveHeader(header.id)}
            >
              <Trash2 className="h-4 w-4"/>
            </Button>
          </div>
        ))}
      </CardContent>
      <CardFooter>
        <Button variant="outline" size="sm" onClick={onAddHeader}>
          Add Header
        </Button>
      </CardFooter>
    </Card>
  );
}
