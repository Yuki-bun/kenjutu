import { openUrl } from "@tauri-apps/plugin-opener"
import { ClipboardCopy, Github } from "lucide-react"
import { useEffect, useState } from "react"

import { commands } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { useGithub } from "@/context/GithubContext"
import { useRpcMutation } from "@/hooks/useRpcQuery"
import { cn } from "@/lib/utils"
export function DeviceAuth() {
  const { isAuthenticated } = useGithub()
  const [deviceCode, setDeviceCode] = useState<{
    userCode: string
    verificationUri: string
  } | null>(null)
  const [copied, setCopied] = useState(false)

  const authMutation = useRpcMutation({
    mutationFn: () => commands.authGithub(),
    onSuccess: (data) => {
      setDeviceCode(data)
    },
  })

  const isNotAuthenticated = !isAuthenticated
  const isAuthenticating = authMutation.isPending

  // Close dialog when authentication completes
  useEffect(() => {
    if (isAuthenticated && deviceCode) {
      setDeviceCode(null)
    }
  }, [isAuthenticated, deviceCode])

  const handleCopy = async () => {
    if (!deviceCode) return
    await navigator.clipboard.writeText(deviceCode.userCode)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <>
      {isNotAuthenticated && (
        <Button
          onClick={() => authMutation.mutate(undefined)}
          disabled={isAuthenticating}
          className={cn(
            "bg-[#24292f] text-white hover:bg-[#24292f]/90",
            "gap-2 shadow-sm",
          )}
        >
          <Github className="h-4 w-4" />
          <span className="hidden sm:inline">
            {isAuthenticating ? "Signing in..." : "Sign in with GitHub"}
          </span>
        </Button>
      )}

      <Dialog
        open={deviceCode !== null}
        onOpenChange={(open) => {
          if (!open) setDeviceCode(null)
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Sign in to GitHub</DialogTitle>
            <DialogDescription>
              Enter this code on GitHub to authorize Revue
            </DialogDescription>
          </DialogHeader>
          <div className="flex items-center justify-center gap-3 py-4">
            <code className="rounded-md bg-muted px-4 py-3 text-2xl font-mono font-bold tracking-widest">
              {deviceCode?.userCode}
            </code>
            <Button variant="outline" size="icon" onClick={handleCopy}>
              <ClipboardCopy className="h-4 w-4" />
              <span className="sr-only">{copied ? "Copied" : "Copy code"}</span>
            </Button>
          </div>
          <p className="text-muted-foreground text-center text-sm">
            A browser window should have opened automatically.
            <br />
            Waiting for authorization...
          </p>
          <DialogFooter showCloseButton>
            <Button
              variant="outline"
              onClick={() => {
                if (deviceCode) {
                  openUrl(deviceCode.verificationUri)
                }
              }}
            >
              Open GitHub
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
