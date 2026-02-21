import {
  createContext,
  RefObject,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { usePaneManager } from "./PaneManager"

const PANEL_KEY_ATTR = "data-panel-key"
const SCROLL_FOCUS_ID_ATTR = "data-scroll-focus-id"
const FOCUSED_ATTR = "data-focused"

interface PaneContext {
  focusedId: string | null
  setFocusedId: (id: string | null) => void
  register: (id: string, ref: RefObject<HTMLElement | null>) => void
  unregister: (id: string) => void
  focusNext: () => void
  focusPrevious: () => void
  suppressNavigation: boolean
  setSuppressNavigation: (suppress: boolean) => void
}

const PaneContext = createContext<PaneContext | null>(null)

export function usePaneContext() {
  const context = useContext(PaneContext)
  if (!context) {
    throw new Error("useScrollFocusContext must be used within a ScrollFocus")
  }
  return context
}

type PaneProps = {
  children: React.ReactNode
  className?: string
  panelKey: string
}

type ScrollFocusEntry = {
  id: string
  element: HTMLElement
  isVisible: boolean
}

export function Pane({ children, className, panelKey }: PaneProps) {
  const { registerPane, unregisterPane, focusPane } = usePaneManager()
  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const [focusedId, setFocusedIdState] = useState<string | null>(null)
  const [suppressNavigation, setSuppressNavigation] = useState(false)
  const entriesRef = useRef<Map<string, ScrollFocusEntry>>(new Map())
  const scrollDirectionRef = useRef<"up" | "down">("down")
  const lastScrollY = useRef<number>(0)
  const observerRef = useRef<IntersectionObserver | null>(null)
  const lastFocusedIdRef = useRef<string | null>(null)

  const onFocus = useCallback(() => {
    const lastFocusedId = lastFocusedIdRef.current
    if (lastFocusedId) {
      const lastFocused = entriesRef.current.get(lastFocusedId)?.element
      if (lastFocused) {
        lastFocused.focus()
        return
      }
    }
    // If no last focused item, focus the first visible item or the first item in the pane.
    const firstVisible = Array.from(entriesRef.current.values()).find(
      (e) => e.isVisible,
    )?.element
    if (firstVisible) {
      firstVisible.focus()
      return
    }
    const firstItem = entriesRef.current.values().next().value?.element
    if (firstItem) {
      firstItem.focus()
      return
    }
    console.warn(
      `Pane with key "${panelKey}" was focused but it has no focusable items.`,
    )
  }, [panelKey])

  const onFocusItem = useCallback((itemKey: string) => {
    const entry = entriesRef.current.get(itemKey)
    if (entry?.element) {
      entry.element.focus()
    }
  }, [])

  const onSoftFocusItem = useCallback((itemKey: string) => {
    const entry = entriesRef.current.get(itemKey)
    if (entry?.element) {
      const lastFocused = entriesRef.current.get(
        lastFocusedIdRef.current ?? "",
      )?.element
      if (lastFocused) {
        lastFocused.removeAttribute(FOCUSED_ATTR)
      }
      lastFocusedIdRef.current = itemKey
      entry.element.setAttribute(FOCUSED_ATTR, "true")
    }
  }, [])

  // Register this pane with the PaneManager
  useEffect(() => {
    registerPane(panelKey, {
      container: scrollContainerRef.current!,
      onFocus,
      onFocusItem,
      onSoftFocusItem,
    })
    return () => unregisterPane(panelKey)
  }, [
    panelKey,
    registerPane,
    unregisterPane,
    focusPane,
    onFocus,
    onSoftFocusItem,
    onFocusItem,
  ])

  // Unregister on unmount
  useEffect(() => {
    return () => {
      unregisterPane(panelKey)
    }
  }, [panelKey, unregisterPane])

  // Track scroll direction via scroll events
  useEffect(() => {
    const container = scrollContainerRef?.current ?? window
    const handleScroll = () => {
      const currentY = scrollContainerRef?.current?.scrollTop ?? window.scrollY
      scrollDirectionRef.current =
        currentY > lastScrollY.current ? "down" : "up"
      lastScrollY.current = currentY
    }
    container.addEventListener("scroll", handleScroll, { passive: true })
    return () => container.removeEventListener("scroll", handleScroll)
  }, [scrollContainerRef])

  // Stable intersection callback using refs
  useEffect(() => {
    const handleVisibilityChange = (entries: IntersectionObserverEntry[]) => {
      entries.forEach((entry) => {
        const id = entry.target.getAttribute(SCROLL_FOCUS_ID_ATTR)
        if (id && entriesRef.current.has(id)) {
          const item = entriesRef.current.get(id)!
          item.isVisible = entry.isIntersecting
        }
      })

      setFocusedIdState((currentFocusedId) => {
        if (!currentFocusedId) return null

        const focused = entriesRef.current.get(currentFocusedId)
        if (focused && !focused.isVisible) {
          const direction = scrollDirectionRef.current
          const visibleEntries = Array.from(entriesRef.current.values())
            .filter((e) => e.isVisible && e.element)
            .sort((a, b) => {
              const rectA = a.element.getBoundingClientRect()
              const rectB = b.element.getBoundingClientRect()
              return (rectA?.top ?? 0) - (rectB?.top ?? 0)
            })

          if (visibleEntries.length === 0) return currentFocusedId

          const nextEntry =
            direction === "down"
              ? visibleEntries[0]
              : visibleEntries[visibleEntries.length - 1]

          if (nextEntry && nextEntry.id !== currentFocusedId) {
            nextEntry.element.focus()
            return nextEntry.id
          }
        }
        return currentFocusedId
      })
    }

    observerRef.current = new IntersectionObserver(handleVisibilityChange, {
      root: scrollContainerRef?.current ?? null,
      threshold: 0,
    })

    // Catch up on entries registered before the observer was created
    for (const [id, entry] of entriesRef.current) {
      if (entry.element) {
        entry.element.setAttribute(SCROLL_FOCUS_ID_ATTR, id)
        observerRef.current.observe(entry.element)
      }
    }

    return () => {
      observerRef.current?.disconnect()
    }
  }, [scrollContainerRef])

  const register = useCallback(
    (id: string, ref: RefObject<HTMLElement | null>) => {
      const element = ref.current
      if (element == null) {
        console.warn(
          `Trying to register scroll focus item with id "${id}" but ref is not attached to an element.`,
        )
        return
      }
      entriesRef.current.set(id, { id, element, isVisible: false })
      if (ref.current && observerRef.current) {
        ref.current.setAttribute(SCROLL_FOCUS_ID_ATTR, id)
        observerRef.current.observe(ref.current)
      }
    },
    [],
  )

  const unregister = useCallback((id: string) => {
    const entry = entriesRef.current.get(id)
    if (entry?.element && observerRef.current) {
      observerRef.current.unobserve(entry.element)
    }
    entriesRef.current.delete(id)
  }, [])

  const setFocusedId = useCallback((id: string | null) => {
    setFocusedIdState(id)
    if (id == null) {
      return
    }
    const lastFocused = entriesRef.current.get(
      lastFocusedIdRef.current ?? "",
    )?.element
    lastFocused?.removeAttribute(FOCUSED_ATTR)

    const newFocused = entriesRef.current.get(id)?.element
    if (!newFocused) {
      console.warn(`Trying to focus item with id "${id}" but no element found.`)
      return
    }
    lastFocusedIdRef.current = id
    newFocused.setAttribute(FOCUSED_ATTR, "true")
  }, [])

  const getSortedEntries = () => {
    return Array.from(entriesRef.current.values())
      .filter((e) => e.element)
      .sort((a, b) => {
        const rectA = a.element.getBoundingClientRect()
        const rectB = b.element.getBoundingClientRect()
        return (rectA?.top ?? 0) - (rectB?.top ?? 0)
      })
  }

  const focusNext = () => {
    const sortedEntries = getSortedEntries()
    const currentIndex = sortedEntries.findIndex((e) => e.id === focusedId)

    // Move to next if not at end
    if (currentIndex >= 0 && currentIndex < sortedEntries.length - 1) {
      const next = sortedEntries[currentIndex + 1]
      next.element.focus()
      next.element.scrollIntoView({
        behavior: "instant",
        block: "nearest",
      })
    }
  }

  const focusPrevious = () => {
    const sortedEntries = getSortedEntries()
    const currentIndex = sortedEntries.findIndex((e) => e.id === focusedId)

    // Move to previous if not at start
    if (currentIndex > 0) {
      const previous = sortedEntries[currentIndex - 1]
      previous.element.focus()
      previous.element.scrollIntoView({
        behavior: "instant",
        block: "nearest",
      })
    }
  }

  const hasFocusedItem = focusedId !== null

  useHotkeys(
    "shift+j",
    () => {
      scrollContainerRef.current?.scrollBy({ top: 100, behavior: "instant" })
    },
    { enabled: hasFocusedItem },
  )
  useHotkeys(
    "shift+k",
    () => {
      scrollContainerRef.current?.scrollBy({ top: -100, behavior: "instant" })
    },
    { enabled: hasFocusedItem },
  )

  useHotkeys("j", focusNext, {
    enabled: hasFocusedItem && !suppressNavigation,
  })
  useHotkeys("k", focusPrevious, {
    enabled: hasFocusedItem && !suppressNavigation,
  })

  return (
    <div
      ref={scrollContainerRef}
      className={className}
      {...(panelKey ? { [PANEL_KEY_ATTR]: panelKey } : {})}
    >
      <PaneContext.Provider
        value={{
          focusedId,
          setFocusedId,
          register,
          unregister,
          focusNext,
          focusPrevious,
          suppressNavigation,
          setSuppressNavigation,
        }}
      >
        {children}
      </PaneContext.Provider>
    </div>
  )
}
