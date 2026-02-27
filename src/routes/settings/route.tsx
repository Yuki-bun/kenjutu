import { createFileRoute } from "@tanstack/react-router"
import { open } from "@tauri-apps/plugin-dialog"
import { useCallback, useState } from "react"

import {
  commands,
  type Error as CommandError,
  type SshSettings,
} from "@/bindings"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { useRpcMutation, useRpcQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
})

function SettingsPage() {
  return (
    <div className="flex flex-col gap-4 p-4 max-w-2xl mx-auto">
      <h1 className="text-2xl font-semibold">Settings</h1>
      <SshSettingsSection />
    </div>
  )
}

function SshSettingsSection() {
  const {
    data: settings,
    error: loadError,
    isLoading,
  } = useRpcQuery({
    queryKey: queryKeys.sshSettings(),
    queryFn: () => commands.getSshSettings(),
  })

  if (isLoading) {
    return <p className="text-muted-foreground">Loading settings...</p>
  }

  if (loadError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>Error</AlertTitle>
        <AlertDescription>Failed to load SSH settings</AlertDescription>
      </Alert>
    )
  }

  return <SshSettingsForm settings={settings!} />
}

function SshSettingsForm({ settings }: { settings: SshSettings }) {
  const [keyPath, setKeyPath] = useState(settings.privateKeyPath ?? "")

  const saveMutation = useRpcMutation<null, CommandError, SshSettings, unknown>(
    {
      mutationFn: (newSettings) => commands.setSshSettings(newSettings),
    },
  )

  const handleBrowse = useCallback(async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      title: "Select SSH Private Key",
    })
    if (selected) {
      setKeyPath(selected)
    }
  }, [])

  const handleSave = useCallback(() => {
    saveMutation.mutate({
      privateKeyPath: keyPath.trim() || null,
    })
  }, [keyPath, saveMutation])

  const handleClear = useCallback(() => {
    setKeyPath("")
    saveMutation.mutate({ privateKeyPath: null })
  }, [saveMutation])

  const isDirty = (keyPath.trim() || null) !== (settings.privateKeyPath ?? null)

  return (
    <Card>
      <CardHeader>
        <h2 className="text-lg font-medium">SSH Authentication</h2>
        <p className="text-sm text-muted-foreground">
          Kenjutu auto-detects SSH credentials when fetching commits from
          remotes. It tries the SSH agent first, then default key files in
          ~/.ssh/. You can optionally override with a specific key path.
        </p>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <label className="text-sm font-medium">
            Private Key Path (optional override)
          </label>
          <div className="flex gap-2">
            <Input
              value={keyPath}
              onChange={(e) => setKeyPath(e.target.value)}
              placeholder="Auto-detect (SSH agent, ~/.ssh/id_ed25519, ...)"
              className="flex-1"
            />
            <Button variant="outline" onClick={handleBrowse}>
              Browse
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            Leave empty to use auto-detection. If set, this key is tried first
            before falling back to the SSH agent and default keys.
          </p>
        </div>

        <div className="flex gap-2">
          <Button
            onClick={handleSave}
            disabled={!isDirty || saveMutation.isPending}
          >
            {saveMutation.isPending ? "Saving..." : "Save"}
          </Button>
          {settings.privateKeyPath && (
            <Button variant="outline" onClick={handleClear}>
              Clear Override
            </Button>
          )}
        </div>

        {saveMutation.isSuccess && (
          <p className="text-sm text-green-600">Settings saved.</p>
        )}
        {saveMutation.isError && (
          <Alert variant="destructive">
            <AlertDescription>Failed to save settings</AlertDescription>
          </Alert>
        )}
      </CardContent>
    </Card>
  )
}
