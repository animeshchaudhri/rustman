import { useState } from "react";
import { AuthType, ApiKeyLocation } from "../types";
import { parseJwt } from "../utils";
import Editor from "@monaco-editor/react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";

interface AuthenticationProps {
  authType: AuthType;
  onAuthTypeChange: (type: AuthType) => void;
  bearerToken: string;
  onBearerTokenChange: (token: string) => void;
  basicUser: string;
  onBasicUserChange: (user: string) => void;
  basicPass: string;
  onBasicPassChange: (pass: string) => void;
  apiKeyName: string;
  onApiKeyNameChange: (name: string) => void;
  apiKeyValue: string;
  onApiKeyValueChange: (value: string) => void;
  apiKeyLocation: ApiKeyLocation;
  onApiKeyLocationChange: (location: ApiKeyLocation) => void;
  userDetail: string;
  onJwtTokenChange: (token: string) => void;
}

export function Authentication({
  authType,
  onAuthTypeChange,
  bearerToken,
  onBearerTokenChange,
  basicUser,
  onBasicUserChange,
  basicPass,
  onBasicPassChange,
  apiKeyName,
  onApiKeyNameChange,
  apiKeyValue,
  onApiKeyValueChange,
  apiKeyLocation,
  onApiKeyLocationChange,
  userDetail,
  onJwtTokenChange
}: AuthenticationProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm">Authentication</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <Select 
          value={authType} 
          onValueChange={(val: AuthType) => onAuthTypeChange(val)}
        >
          <SelectTrigger>
            <SelectValue placeholder="Auth Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">No Auth</SelectItem>
            <SelectItem value="basic">Basic Auth</SelectItem>
            <SelectItem value="bearer">Bearer Token</SelectItem>
            <SelectItem value="apikey">API Key</SelectItem>
            <SelectItem value="jwt-user">JWT to x-user-detail</SelectItem>
          </SelectContent>
        </Select>

        {authType === 'basic' && (
          <div className="grid grid-cols-2 gap-4">
            <Input 
              placeholder="Username" 
              value={basicUser} 
              onChange={e => onBasicUserChange(e.target.value)}
            />
            <Input 
              type="password" 
              placeholder="Password" 
              value={basicPass} 
              onChange={e => onBasicPassChange(e.target.value)}
            />
          </div>
        )}

        {authType === 'bearer' && (
          <Textarea 
            placeholder="Bearer Token" 
            value={bearerToken} 
            onChange={e => onBearerTokenChange(e.target.value)} 
            className="font-mono"
          />
        )}

        {authType === 'apikey' && (
          <div className="space-y-2">
            <div className="grid grid-cols-2 gap-4">
              <Input 
                placeholder="Header Name / Query Param Name" 
                value={apiKeyName} 
                onChange={e => onApiKeyNameChange(e.target.value)}
              />
              <Select 
                value={apiKeyLocation} 
                onValueChange={(val: ApiKeyLocation) => onApiKeyLocationChange(val)}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Add to"/>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="header">Header</SelectItem>
                  <SelectItem value="query">Query Param</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <Textarea 
              placeholder="API Key Value" 
              value={apiKeyValue} 
              onChange={e => onApiKeyValueChange(e.target.value)} 
              className="font-mono"
            />
          </div>
        )}

        {authType === 'jwt-user' && (
          <div className="space-y-4">
            <div>
              <Label>JWT Token</Label>
              <Textarea 
                placeholder="Paste JWT token here" 
                value={bearerToken} 
                onChange={e => onJwtTokenChange(e.target.value)}
                className="font-mono mt-1"
              />
            </div>
            <div>
              <Label>Parsed x-user-detail</Label>
              <Editor 
                height="200px" 
                language="json" 
                value={userDetail} 
                theme="vs-dark"
                options={{
                  readOnly: true,
                  minimap: { enabled: false },
                  lineNumbers: 'off'
                }}
              />
            </div>
          </div>
        )}

        {authType !== 'none' && (
          <p className="text-xs text-muted-foreground">
            Authentication headers/params will be added automatically when you send the request.
          </p>
        )}
      </CardContent>
    </Card>
  );
}
