import { useHotkey } from "@tanstack/react-hotkeys"
import { Send, X } from "lucide-react"
import { useRef, useState } from "react"

import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"

import type { InlineCommentFormProps } from "./Diff"

export function InlineCommentForm({
  onSubmit,
  onCancel,
  placeholder = "Write a comment...",
}: InlineCommentFormProps) {
  const [body, setBody] = useState("")

  const handleSubmit = () => {
    const trimmed = body.trim()
    if (!trimmed) return
    onSubmit(trimmed)
    setBody("")
  }

  const textAreaRef = useRef<HTMLTextAreaElement>(null)
  useHotkey("Meta+Enter", handleSubmit, { target: textAreaRef })
  useHotkey("Escape", onCancel, { target: textAreaRef })

  return (
    <div className="flex flex-col gap-2 p-3">
      <Textarea
        ref={textAreaRef}
        value={body}
        onChange={(e) => setBody(e.target.value)}
        placeholder={placeholder}
        autoFocus
        className="min-h-[60px] text-xs"
      />
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">
          {navigator.platform.includes("Mac") ? "Cmd" : "Ctrl"}+Enter to submit
        </span>
        <div className="flex gap-1">
          <Button variant="ghost" size="xs" onClick={onCancel}>
            <X className="w-3 h-3" />
            Cancel
          </Button>
          <Button size="xs" disabled={!body.trim()} onClick={handleSubmit}>
            <Send className="w-3 h-3" />
            Comment
          </Button>
        </div>
      </div>
    </div>
  )
}
