import { RefObject, useEffect, useRef, useState } from "react"

type ScrollFocusEntry = {
  id: string
  ref: RefObject<HTMLElement | null>
  isVisible: boolean
}

type UseScrollFocusOptions = {
  scrollContainerRef?: RefObject<HTMLElement | null>
}

const SCROLL_FOCUS_ID_ATTR = "data-scroll-focus-id"

export function useScrollFocus(options?: UseScrollFocusOptions) {
  const { scrollContainerRef } = options ?? {}

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
            .filter((e) => e.isVisible && e.ref.current)
            .sort((a, b) => {
              const rectA = a.ref.current?.getBoundingClientRect()
              const rectB = b.ref.current?.getBoundingClientRect()
              return (rectA?.top ?? 0) - (rectB?.top ?? 0)
            })

          if (visibleEntries.length === 0) return currentFocusedId

          const nextEntry =
            direction === "down"
              ? visibleEntries[0]
              : visibleEntries[visibleEntries.length - 1]

          if (nextEntry && nextEntry.id !== currentFocusedId) {
            nextEntry.ref.current?.focus()
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

    return () => {
      observerRef.current?.disconnect()
    }
  }, [scrollContainerRef])

  const register = (id: string, ref: RefObject<HTMLElement | null>) => {
    entriesRef.current.set(id, { id, ref, isVisible: false })
    if (ref.current && observerRef.current) {
      ref.current.setAttribute(SCROLL_FOCUS_ID_ATTR, id)
      observerRef.current.observe(ref.current)
    }
  }

  const unregister = (id: string) => {
    const entry = entriesRef.current.get(id)
    if (entry?.ref.current && observerRef.current) {
      observerRef.current.unobserve(entry.ref.current)
    }
    entriesRef.current.delete(id)
  }

  const setFocusedId = (id: string | null) => {
    setFocusedIdState(id)
  }

  return {
    focusedId,
    setFocusedId,
    register,
    unregister,
  }
}
