// Test wrapper components for ScrollFocus Playwright tests
// These must be in a separate file from the test file per Playwright CT requirements

import { useState } from "react"

import { ScrollFocus, useScrollFocusItem } from "./ScrollFocus"

// =============================================================================
// Test Item Component
// =============================================================================

interface TestItemProps {
  id: string
  height?: number
}

export function TestItem({ id, height = 60 }: TestItemProps) {
  const { ref, isFocused } = useScrollFocusItem<HTMLDivElement>(id)
  return (
    <div
      ref={ref}
      tabIndex={0}
      data-testid={id}
      className="border-b border-gray-200 flex items-center px-4"
      style={{ height }}
    >
      <span>{id}</span>
      {isFocused && <span data-testid={`${id}-focused-indicator`}> ✓</span>}
    </div>
  )
}

// =============================================================================
// Test List Component
// =============================================================================

interface TestListProps {
  panelKey?: string
  itemCount?: number
  itemHeight?: number
}

export function TestList({
  panelKey = "test-panel",
  itemCount = 10,
  itemHeight = 60,
}: TestListProps) {
  return (
    <ScrollFocus panelKey={panelKey} className="scroll-container">
      {Array.from({ length: itemCount }, (_, i) => (
        <TestItem key={`item-${i}`} id={`item-${i}`} height={itemHeight} />
      ))}
    </ScrollFocus>
  )
}

// =============================================================================
// Dynamic Test List Component
// =============================================================================

export function DynamicTestList({
  panelKey = "test-panel",
}: {
  panelKey?: string
}) {
  const [itemCount, setItemCount] = useState(3)
  return (
    <div>
      <button
        data-testid="add-items"
        onClick={() => setItemCount((c) => c + 2)}
      >
        Add Items
      </button>
      <ScrollFocus panelKey={panelKey} className="scroll-container">
        {Array.from({ length: itemCount }, (_, i) => (
          <TestItem key={`item-${i}`} id={`item-${i}`} height={60} />
        ))}
      </ScrollFocus>
    </div>
  )
}

// =============================================================================
// Empty Test List Component
// =============================================================================

export function EmptyTestList({
  panelKey = "test-panel",
}: {
  panelKey?: string
}) {
  return (
    <ScrollFocus panelKey={panelKey} className="scroll-container">
      <div>{/* intentionally empty */}</div>
    </ScrollFocus>
  )
}
