import { cn } from "@/lib/utils"

export function CommentCard({
  children,
  className,
}: {
  children: React.ReactNode
  className?: string
}) {
  return (
    <div className={cn("rounded-lg border bg-card", className)}>{children}</div>
  )
}

export function CommentCardHeader({
  children,
  className,
}: {
  children: React.ReactNode
  className?: string
}) {
  return (
    <div className={cn("border-b bg-muted/30 py-2 px-4", className)}>
      {children}
    </div>
  )
}

export function CommentCardContent({
  children,
  className,
}: {
  children: React.ReactNode
  className?: string
}) {
  return <div className={cn("px-4 py-3", className)}>{children}</div>
}
