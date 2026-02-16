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

interface ScrollFocusContextValue {
  focusedId: string | null
  setFocusedId: (id: string | null) => void
  register: (id: string, ref: RefObject<HTMLElement | null>) => void
  unregister: (id: string) => void
  focusNext: () => void
  focusPrevious: () => void
}

const ScrollFocusContext = createContext<ScrollFocusContextValue | null>(null)

export function useScrollFocusContext() {
  const context = useContext(ScrollFocusContext)
  if (!context) {
    throw new Error("useScrollFocusContext must be used within a ScrollFocus")
  }
  return context
}

type ScrollFocusProps = {
  children: React.ReactNode
  className?: string
  panelKey: string
}

export function ScrollFocus({
  children,
  className,
  panelKey,
}: ScrollFocusProps) {
  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const [focusedId, setFocusedIdState] = useState<string | null>(null)
  const entriesRef = useRef<Map<string, ScrollFocusEntry>>(new Map())
  const scrollDirectionRef = useRef<"up" | "down">("down")
  const lastScrollY = useRef<number>(0)
  const observerRef = useRef<IntersectionObserver | null>(null)

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

  const setFocusedId = useCallback(
    (id: string | null) => {
      setFocusedIdState(id)
      if (id == null) {
        return
      }
      const lastFocusedElement = document
        .querySelector(`[${PANEL_KEY_ATTR}="${panelKey}"]`)
        ?.querySelector(`[${FOCUSED_ATTR}]`)
      lastFocusedElement?.removeAttribute(FOCUSED_ATTR)
      const newElement = entriesRef.current.get(id)?.element
      newElement?.setAttribute(FOCUSED_ATTR, "true")
    },
    [panelKey],
  )

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

  useHotkeys("j", focusNext, { enabled: hasFocusedItem })
  useHotkeys("k", focusPrevious, { enabled: hasFocusedItem })

  return (
    <div
      ref={scrollContainerRef}
      className={className}
      {...(panelKey ? { [PANEL_KEY_ATTR]: panelKey } : {})}
    >
      <ScrollFocusContext.Provider
        value={{
          focusedId,
          setFocusedId,
          register,
          unregister,
          focusNext,
          focusPrevious,
        }}
      >
        {children}
      </ScrollFocusContext.Provider>
    </div>
  )
}

type UseScrollFocusItemOptions = {
  onFocus?: () => void
}

export function useScrollFocusItem<T extends HTMLElement = HTMLElement>(
  id: string,
  { onFocus }: UseScrollFocusItemOptions = {},
) {
  const ref = useRef<T>(null)
  const { focusedId, setFocusedId, register, unregister } =
    useScrollFocusContext()

  useEffect(() => {
    register(id, ref)
    return () => unregister(id)
  }, [id, register, unregister])

  // Auto-attach focus/blur listeners
  // Event order is guaranteed by spec: blur fires before focus
  // https://w3c.github.io/uievents/#events-focusevent-event-order
  // So when moving from A to B: A blurs (null) → B focuses (B's id)
  useEffect(() => {
    const element = ref.current
    if (!element) return

    const handleFocus = () => {
      setFocusedId(id)
      onFocus?.()
    }
    const handleBlur = () => setFocusedId(null)

    element.addEventListener("focus", handleFocus)
    element.addEventListener("blur", handleBlur)
    return () => {
      element.removeEventListener("focus", handleFocus)
      element.removeEventListener("blur", handleBlur)
    }
  }, [id, onFocus, setFocusedId])

  const isFocused = focusedId === id

  const scrollIntoView = () => {
    const element = ref.current
    if (element) {
      element.scrollIntoView(true)
    }
  }

  return { ref, isFocused, scrollIntoView }
}

type ScrollFocusEntry = {
  id: string
  element: HTMLElement
  isVisible: boolean
}

const PANEL_KEY_ATTR = "data-panel-key"
const SCROLL_FOCUS_ID_ATTR = "data-scroll-focus-id"
const FOCUSED_ATTR = "data-focused"

export function focusPanel(panelKey: string) {
  const container = document.querySelector(`[${PANEL_KEY_ATTR}="${panelKey}"]`)
  if (!container) return
  const lastFocusedItem = container.querySelector(`[${FOCUSED_ATTR}]`)
  if (lastFocusedItem instanceof HTMLElement) {
    lastFocusedItem.focus()
    return
  }
  const firstItem = container.querySelector(`[${SCROLL_FOCUS_ID_ATTR}]`)
  if (firstItem instanceof HTMLElement) {
    firstItem.focus()
  }
}

export function focusItemInPanel(panelKey: string, itemId: string) {
  const container = document.querySelector(`[${PANEL_KEY_ATTR}="${panelKey}"]`)
  if (!container) return
  const item = container.querySelector(`[${SCROLL_FOCUS_ID_ATTR}="${itemId}"]`)
  if (item instanceof HTMLElement) {
    item.scrollIntoView({ behavior: "instant", block: "start" })
    item.focus()
  }
}

export function softFocusItemInPanel(panelKey: string, itemId: string) {
  const container = document.querySelector(`[${PANEL_KEY_ATTR}="${panelKey}"]`)
  if (!container) return
  const item = container.querySelector(`[${SCROLL_FOCUS_ID_ATTR}="${itemId}"]`)
  if (item instanceof HTMLElement) {
    const lastFocusedItem = container.querySelector(`[${FOCUSED_ATTR}]`)
    lastFocusedItem?.removeAttribute(FOCUSED_ATTR)
    item.setAttribute(FOCUSED_ATTR, "true")
  }
}

export const PANEL_KEYS = {
  diffVew: "diff-view",
  fileTree: "file-tree",
  prCommitList: "pr-commit-list",
  commitGraph: "commit-graph",
} as const
