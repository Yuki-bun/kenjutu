import { Loader2, Send, X } from "lucide-react"
import { useState } from "react"

import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"

type InlineCommentFormProps = {
  onSubmit: (body: string) => void
  onCancel: () => void
  isPending: boolean
  placeholder?: string
}

export function InlineCommentForm({
  onSubmit,
  onCancel,
  isPending,
  placeholder = "Write a comment...",
}: InlineCommentFormProps) {
  const [body, setBody] = useState("")

  const handleSubmit = () => {
    const trimmed = body.trim()
    if (!trimmed) return
    onSubmit(trimmed)
    setBody("")
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault()
      handleSubmit()
    }
    if (e.key === "Escape") {
      e.preventDefault()
      onCancel()
    }
  }

  return (
    <div className="flex flex-col gap-2 p-3">
      <Textarea
        value={body}
        onChange={(e) => setBody(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        disabled={isPending}
        autoFocus
        className="min-h-[60px] text-xs"
      />
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">
          {navigator.platform.includes("Mac") ? "Cmd" : "Ctrl"}+Enter to submit
        </span>
        <div className="flex gap-1">
          <Button
            variant="ghost"
            size="xs"
            onClick={onCancel}
            disabled={isPending}
          >
            <X className="w-3 h-3" />
            Cancel
          </Button>
          <Button
            size="xs"
            disabled={isPending || !body.trim()}
            onClick={handleSubmit}
          >
            {isPending ? (
              <Loader2 className="w-3 h-3 animate-spin" />
            ) : (
              <Send className="w-3 h-3" />
            )}
            Comment
          </Button>
        </div>
      </div>
    </div>
  )
}
