import { HeaderType } from "../types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Trash2 } from "lucide-react";

interface RequestParamsProps {
  params: HeaderType[];
  onAddParam: () => void;
  onParamChange: (id: string, field: "key" | "value" | "enabled", value: string | boolean) => void;
  onRemoveParam: (id: string) => void;
}

export function RequestParams({ 
  params, 
  onAddParam, 
  onParamChange, 
  onRemoveParam 
}: RequestParamsProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm">Query Parameters</CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {params.map((param) => (
          <div key={param.id} className="flex gap-2 items-center">
            <Input 
              type="checkbox" 
              checked={param.enabled} 
              onChange={(e) => onParamChange(param.id, "enabled", e.target.checked)} 
              className="w-5 h-5"
            />
            <Input 
              placeholder="Key" 
              value={param.key} 
              onChange={(e) => onParamChange(param.id, "key", e.target.value)} 
              disabled={!param.enabled}
            />
            <Input 
              placeholder="Value" 
              value={param.value} 
              onChange={(e) => onParamChange(param.id, "value", e.target.value)} 
              disabled={!param.enabled}
            />
            <Button 
              variant="ghost" 
              size="icon" 
              onClick={() => onRemoveParam(param.id)}
            >
              <Trash2 className="h-4 w-4"/>
            </Button>
          </div>
        ))}
      </CardContent>
      <CardFooter>
        <Button variant="outline" size="sm" onClick={onAddParam}>
          Add Parameter
        </Button>
      </CardFooter>
    </Card>
  );
}
