import { AuthType, ApiKeyLocation, CookieType } from "../types";
import Editor from "@monaco-editor/react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { useTheme } from "@/contexts/ThemeContext";

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
  cookieString: string;
  onCookieStringChange: (cookies: string) => void;
  cookies: CookieType[];
  onAddCookie: () => void;
  onCookieChange: (id: string, field: "name" | "value" | "enabled", value: string | boolean) => void;
  onRemoveCookie: (id: string) => void;
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
  onJwtTokenChange,
  cookieString,
  onCookieStringChange,
  cookies,
  onAddCookie,
  onCookieChange,
  onRemoveCookie,
}: AuthenticationProps) {
  const { resolved } = useTheme();

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm">Authentication</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <Select value={authType} onValueChange={(val: AuthType) => onAuthTypeChange(val)}>
          <SelectTrigger>
            <SelectValue placeholder="Auth Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">No Auth</SelectItem>
            <SelectItem value="basic">Basic Auth</SelectItem>
            <SelectItem value="bearer">Bearer Token</SelectItem>
            <SelectItem value="apikey">API Key</SelectItem>
            <SelectItem value="jwt-user">JWT to x-user-detail</SelectItem>
            <SelectItem value="cookie">Cookies</SelectItem>
          </SelectContent>
        </Select>

        {authType === "basic" && (
          <div className="grid grid-cols-2 gap-4">
            <Input
              placeholder="Username"
              value={basicUser}
              onChange={(e) => onBasicUserChange(e.target.value)}
              autoCorrect="off"
              autoCapitalize="none"
              spellCheck={false}
            />
            <Input
              type="password"
              placeholder="Password"
              value={basicPass}
              onChange={(e) => onBasicPassChange(e.target.value)}
              autoCorrect="off"
              autoCapitalize="none"
              spellCheck={false}
              autoComplete="current-password"
            />
          </div>
        )}

        {authType === "bearer" && (
          <Textarea
            placeholder="Bearer Token"
            value={bearerToken}
            onChange={(e) => onBearerTokenChange(e.target.value)}
            autoCorrect="off"
            autoCapitalize="none"
            spellCheck={false}
            className="font-mono"
          />
        )}

        {authType === "apikey" && (
          <div className="space-y-2">
            <div className="grid grid-cols-2 gap-4">
              <Input
                placeholder="Header Name / Query Param Name"
                value={apiKeyName}
                onChange={(e) => onApiKeyNameChange(e.target.value)}
                autoCorrect="off"
                autoCapitalize="none"
                spellCheck={false}
              />
              <Select value={apiKeyLocation} onValueChange={(val: ApiKeyLocation) => onApiKeyLocationChange(val)}>
                <SelectTrigger>
                  <SelectValue placeholder="Add to" />
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
              onChange={(e) => onApiKeyValueChange(e.target.value)}
              autoCorrect="off"
              autoCapitalize="none"
              spellCheck={false}
              className="font-mono"
            />
          </div>
        )}

        {authType === "jwt-user" && (
          <div className="space-y-4">
            <div>
              <Label>JWT Token</Label>
              <Textarea
                placeholder="Paste JWT token here"
                value={bearerToken}
                onChange={(e) => onJwtTokenChange(e.target.value)}
                autoCorrect="off"
                autoCapitalize="none"
                spellCheck={false}
                className="font-mono mt-1"
              />
            </div>
            <div>
              <Label>Parsed x-user-detail</Label>
              <Editor
                height="200px"
                language="json"
                value={userDetail}
                theme={resolved === "dark" ? "vs-dark" : "vs"}
                options={{
                  readOnly: true,
                  minimap: { enabled: false },
                  lineNumbers: "off",
                }}
              />
            </div>
          </div>
        )}

        {authType === "cookie" && (
          <div className="space-y-4">
            <div>
              <Label>Cookie String</Label>
              <Textarea
                placeholder="Paste cookie string from browser or cURL (e.g., name1=value1; name2=value2)"
                value={cookieString}
                onChange={(e) => onCookieStringChange(e.target.value)}
                autoCorrect="off"
                autoCapitalize="none"
                spellCheck={false}
                className="font-mono mt-1 min-h-[100px]"
              />
            </div>
            <div>
              <div className="flex items-center justify-between mb-2">
                <Label>Individual Cookies</Label>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={onAddCookie}
                >
                  Add Cookie
                </Button>
              </div>
              <div className="space-y-2 max-h-[200px] overflow-y-auto">
                {cookies.map((cookie) => (
                  <div key={cookie.id} className="flex items-center gap-2">
                    <Input
                      placeholder="Name"
                      value={cookie.name}
                      onChange={(e) => onCookieChange(cookie.id, "name", e.target.value)}
                      autoCorrect="off"
                      autoCapitalize="none"
                      spellCheck={false}
                      className="flex-1"
                    />
                    <Input
                      placeholder="Value"
                      value={cookie.value}
                      onChange={(e) => onCookieChange(cookie.id, "value", e.target.value)}
                      autoCorrect="off"
                      autoCapitalize="none"
                      spellCheck={false}
                      className="flex-1"
                    />
                    <Input
                      type="checkbox"
                      checked={cookie.enabled}
                      onChange={(e) => onCookieChange(cookie.id, "enabled", e.target.checked)}
                      className="w-4 h-4"
                    />
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => onRemoveCookie(cookie.id)}
                    >
                      ✕
                    </Button>
                  </div>
                ))}
              </div>
            </div>
            {bearerToken && (
              <div>
                <Label>Extracted Access Token</Label>
                <Textarea
                  value={bearerToken}
                  readOnly
                  className="font-mono mt-1 bg-muted text-muted-foreground"
                />
              </div>
            )}
            {userDetail && (
              <div>
                <Label>Parsed JWT Details</Label>
                <Editor
                  height="150px"
                  language="json"
                  value={userDetail}
                  theme={resolved === "dark" ? "vs-dark" : "vs"}
                  options={{
                    readOnly: true,
                    minimap: { enabled: false },
                    lineNumbers: "off",
                  }}
                />
              </div>
            )}
          </div>
        )}

        {authType !== "none" && (
          <p className="text-xs text-muted-foreground">
            Authentication headers/params will be added automatically when you send the request.
          </p>
        )}
      </CardContent>
    </Card>
  );
}
