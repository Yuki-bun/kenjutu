import { useVirtualizer } from "@tanstack/react-virtual"
import { RefObject, useCallback, useLayoutEffect, useState } from "react"

import { VirtualRow, VirtualRowModel } from "./virtualRows"

function estimateRowSize(row: VirtualRow): number {
  switch (row.type) {
    case "gap":
      return 28
    case "commentForm":
      return 120
    case "unifiedLine":
    case "splitLine":
      return 20
  }
}

function measureScrollMargin(
  container: HTMLDivElement,
  scrollEl: HTMLDivElement,
): number {
  return Math.round(
    container.getBoundingClientRect().top -
      scrollEl.getBoundingClientRect().top +
      scrollEl.scrollTop,
  )
}

export function useVirtualDiff({
  rowModel,
  scrollContainerRef,
  containerRef,
}: {
  rowModel: VirtualRowModel
  scrollContainerRef: RefObject<HTMLDivElement | null>
  containerRef: RefObject<HTMLDivElement | null>
}) {
  const [scrollMargin, setScrollMargin] = useState(0)

  useLayoutEffect(() => {
    const container = containerRef.current
    const scrollEl = scrollContainerRef.current
    if (!container || !scrollEl) return
    setScrollMargin(measureScrollMargin(container, scrollEl))
  }, [containerRef, scrollContainerRef])

  const virtualizer = useVirtualizer({
    count: rowModel.rows.length,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: (index) => estimateRowSize(rowModel.rows[index]),
    overscan: 30,
    scrollMargin,
  })

  const scrollToNavIndex = useCallback(
    (navIndex: number) => {
      if (navIndex < 0 || navIndex >= rowModel.navToVirtual.length) return
      virtualizer.scrollToIndex(rowModel.navToVirtual[navIndex], {
        align: "auto",
      })
    },
    [rowModel.navToVirtual, virtualizer],
  )

  return { virtualizer, scrollToNavIndex }
}
