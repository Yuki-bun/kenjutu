import { MessageSquare } from "lucide-react"

import { cn } from "@/lib/utils"

export function LineNumberGutter({
  className,
  hasComments,
  children,
}: {
  className?: string
  hasComments?: boolean
  children: React.ReactNode
}) {
  return (
    <span
      className={cn(
        "text-right pr-2 text-muted-foreground select-none shrink-0 relative",
        className,
      )}
    >
      {hasComments && (
        <span className="absolute left-0 top-1/2 -translate-y-1/2 inline-flex text-blue-500 rounded-sm p-0.5 z-10">
          <MessageSquare className="w-3 h-3" />
        </span>
      )}
      {children}
    </span>
  )
}
