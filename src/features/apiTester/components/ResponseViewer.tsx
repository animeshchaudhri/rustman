import React from "react";
import { ApiResponse, ResponseBodyView, ResponseTabType } from "../types";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { Loader2, Clock, Database, Check, XCircle } from "lucide-react";
import { cn } from "@/lib/utils";

interface ResponseViewerProps {
  isLoading: boolean;
  response: ApiResponse | null;
  error: string | null;
  responseTime: number | null;
  responseSize: number | null;
  activeTab: string;
  onTabChange: (tab: ResponseTabType) => void;
  bodyView: ResponseBodyView;
  onBodyViewChange: (view: ResponseBodyView) => void;
}

export function ResponseViewer({
  isLoading,
  response,
  error,
  responseTime,
  responseSize,
  activeTab,
  onTabChange,
  bodyView,
  onBodyViewChange,
}: ResponseViewerProps) {
  
  // Format a number in bytes to a human-readable size
  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  // Format response data based on type and view mode
  const formatData = (data: any) => {
    if (typeof data === 'object') {
      return bodyView === 'pretty' 
        ? JSON.stringify(data, null, 2) 
        : JSON.stringify(data);
    }
    return data?.toString() || '';
  };

  // Status color based on HTTP status code
  const getStatusColor = (status?: number) => {
    if (!status) return 'destructive';
    
    if (status >= 200 && status < 300) return 'default'; // Green for success
    if (status >= 300 && status < 400) return 'secondary'; // Blue for redirection
    if (status >= 400 && status < 500) return 'outline'; // Yellow for client errors
    return 'destructive'; // Red for server errors or unknown
  };

  return (
    <div className="flex-1 flex flex-col bg-background overflow-hidden">
      {/* Response Status Bar */}
      <div className="p-3 border-b flex items-center justify-between bg-muted/30">
        <div className="flex items-center gap-3">
          {isLoading ? (
            <div className="flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span className="text-sm font-medium">Loading...</span>
            </div>
          ) : response ? (
            <div className="flex items-center gap-2">
              <Badge variant={getStatusColor(response.status)}>
                {response.status || 0}
              </Badge>
              <span className="text-sm font-medium">
                {response.statusText || (response.status ? `HTTP ${response.status}` : 'No Response')}
              </span>
            </div>
          ) : error ? (
            <div className="flex items-center gap-2 text-destructive">
              <XCircle className="h-4 w-4" />
              <span className="text-sm font-medium">Request Failed</span>
            </div>
          ) : (
            <span className="text-sm text-muted-foreground">Send a request to see response</span>
          )}
        </div>

        {!isLoading && response && (
          <div className="flex items-center gap-4 text-xs text-muted-foreground">
            {responseTime !== null && (
              <div className="flex items-center gap-1">
                <Clock className="h-3.5 w-3.5" />
                <span>{responseTime} ms</span>
              </div>
            )}
            {responseSize !== null && (
              <div className="flex items-center gap-1">
                <Database className="h-3.5 w-3.5" />
                <span>{formatSize(responseSize)}</span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Response Content */}
      <div className="flex-1 overflow-hidden flex flex-col">
        {isLoading ? (
          <div className="flex-1 flex items-center justify-center">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : error ? (
          <div className="flex-1 p-4 flex flex-col items-center justify-center text-destructive">
            <XCircle className="h-12 w-12 mb-2" />
            <h3 className="text-lg font-medium">Request Error</h3>
            <p className="mt-1">{error}</p>
          </div>
        ) : !response ? (
          <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground">
            <Check className="h-12 w-12 mb-2" />
            <p>Send a request to see the response</p>
          </div>
        ) : (
          <Tabs 
            value={activeTab} 
            onValueChange={onTabChange as (value: string) => void}
            className="flex-1 flex flex-col"
          >
            <TabsList className="px-4 py-2 bg-background border-b">
              <TabsTrigger value="body">Body</TabsTrigger>
              <TabsTrigger value="headers">Headers</TabsTrigger>
              <TabsTrigger value="cookies">Cookies</TabsTrigger>
              
              {/* View toggle for response body */}
              {activeTab === 'body' && (
                <div className="ml-auto flex items-center gap-2">
                  <div className="text-xs font-medium text-muted-foreground">View:</div>
                  <div className="flex border rounded-md overflow-hidden">
                    <button
                      onClick={() => onBodyViewChange('pretty')}
                      className={cn(
                        "text-xs px-2 py-1",
                        bodyView === 'pretty' 
                          ? "bg-primary text-primary-foreground" 
                          : "bg-muted hover:bg-muted/80"
                      )}
                    >
                      Pretty
                    </button>
                    <button
                      onClick={() => onBodyViewChange('raw')}
                      className={cn(
                        "text-xs px-2 py-1",
                        bodyView === 'raw' 
                          ? "bg-primary text-primary-foreground" 
                          : "bg-muted hover:bg-muted/80"
                      )}
                    >
                      Raw
                    </button>
                  </div>
                </div>
              )}
            </TabsList>
            
            {/* Body Tab */}
            <TabsContent value="body" className="flex-1 overflow-auto p-0">
              {response.data !== undefined ? (
                <pre className="text-sm p-4 whitespace-pre-wrap font-mono overflow-auto h-full">
                  {formatData(response.data)}
                </pre>
              ) : (
                <div className="flex items-center justify-center h-full text-muted-foreground">
                  No response body
                </div>
              )}
            </TabsContent>
            
            {/* Headers Tab */}
            <TabsContent value="headers" className="flex-1 overflow-auto p-4">
              {response.headers && Object.keys(response.headers).length > 0 ? (
                <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2">
                  {Object.entries(response.headers).map(([key, value]) => (
                    <React.Fragment key={key}>
                      <div className="font-medium text-sm">{key}:</div>
                      <div className="text-sm font-mono break-all">
                        {value}
                      </div>
                    </React.Fragment>
                  ))}
                </div>
              ) : (
                <div className="flex items-center justify-center h-full text-muted-foreground">
                  No headers
                </div>
              )}
            </TabsContent>
            
            {/* Cookies Tab */}
            <TabsContent value="cookies" className="flex-1 overflow-auto p-4">
              {response.cookies ? (
                <div className="font-mono whitespace-pre-wrap break-all text-sm">
                  {response.cookies}
                </div>
              ) : (
                <div className="flex items-center justify-center h-full text-muted-foreground">
                  No cookies
                </div>
              )}
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  );
}
