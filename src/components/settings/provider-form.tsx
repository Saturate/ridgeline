import { useState } from "react";
import { CheckCircle, Loader2, XCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import type { ProviderConfig } from "@/lib/types";

interface ProviderFormProps {
  initial?: ProviderConfig;
  onSave: (provider: ProviderConfig) => void;
  onCancel: () => void;
}

export function ProviderForm({ initial, onSave, onCancel }: ProviderFormProps) {
  const [name, setName] = useState(initial?.name ?? "");
  const [url, setUrl] = useState(initial?.url ?? "");
  const [token, setToken] = useState("");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    message: string;
  } | null>(null);

  const handleTest = async () => {
    if (!name || !url || !token) return;
    setTesting(true);
    setTestResult(null);
    try {
      const displayName = await api.testConnection(name, url, token);
      setTestResult({ ok: true, message: `Connected as ${displayName}` });
    } catch (e) {
      setTestResult({ ok: false, message: String(e) });
    } finally {
      setTesting(false);
    }
  };

  const handleSave = async () => {
    if (token) {
      await api.storeToken(name, token);
    }
    onSave({
      type: "azure-devops",
      name,
      url,
      projects: initial?.projects ?? [],
    });
  };

  const canSave = name && url && (token || initial);

  return (
    <div className="space-y-3 rounded-md border p-3">
      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <label className="mb-1 block text-xs font-medium">Name</label>
          <Input
            placeholder="e.g. contoso"
            value={name}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
          />
        </div>
        <div>
          <label className="mb-1 block text-xs font-medium">
            Organization URL
          </label>
          <Input
            placeholder="https://dev.azure.com/contoso"
            value={url}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setUrl(e.target.value)}
          />
        </div>
      </div>
      <div>
        <label className="mb-1 block text-xs font-medium">
          Personal Access Token
          {initial && (
            <span className="ml-1 text-muted-foreground">(leave empty to keep existing)</span>
          )}
        </label>
        <div className="flex gap-2">
          <Input
            type="password"
            placeholder="Enter PAT"
            value={token}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
              setToken(e.target.value);
              setTestResult(null);
            }}
          />
          <Button
            variant="outline"
            onClick={handleTest}
            disabled={testing || !name || !url || !token}
          >
            {testing && <Loader2 className="mr-1 h-4 w-4 animate-spin" />}
            Test
          </Button>
        </div>
        {testResult && (
          <div
            className={`mt-1 flex items-center gap-1 text-xs ${testResult.ok ? "text-green-600" : "text-red-600"}`}
          >
            {testResult.ok ? (
              <CheckCircle className="h-3 w-3" />
            ) : (
              <XCircle className="h-3 w-3" />
            )}
            {testResult.message}
          </div>
        )}
      </div>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
        <Button size="sm" onClick={handleSave} disabled={!canSave}>
          Save
        </Button>
      </div>
    </div>
  );
}
