import { GripVertical } from "lucide-react"
import * as ResizablePrimitive from "react-resizable-panels"

import { cn } from "@/lib/utils"

const ResizablePanelGroup = ({
  className,
  ...props
}: React.ComponentProps<typeof ResizablePrimitive.Group>) => (
  <ResizablePrimitive.Group
    className={cn(
      "flex h-full w-full data-[panel-group-direction=vertical]:flex-col",
      className,
    )}
    {...props}
  />
)

const ResizablePanel = ResizablePrimitive.Panel

const ResizableHandle = ({
  withHandle,
  className,
  ...props
}: React.ComponentProps<typeof ResizablePrimitive.Separator> & {
  withHandle?: boolean
}) => (
  // ... inside the component
  <ResizablePrimitive.Separator
    className={cn(
      "relative flex w-px items-center justify-center bg-border",
      className,
    )}
    {...props}
  >
    {withHandle && (
      <div className="sticky top-1/2 z-10 flex h-4 w-3 -translate-y-1/2 items-center justify-center rounded-sm border bg-border data-[panel-group-direction=vertical]:left-1/2 data-[panel-group-direction=vertical]:top-auto data-[panel-group-direction=vertical]:-translate-x-1/2 data-[panel-group-direction=vertical]:-translate-y-0">
        <GripVertical className="h-2.5 w-2.5" />
      </div>
    )}
  </ResizablePrimitive.Separator>
)

const usePanelRef = ResizablePrimitive.usePanelRef

export { ResizableHandle, ResizablePanel, ResizablePanelGroup, usePanelRef }
