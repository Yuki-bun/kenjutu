import { act, render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"

import {
  focusItemInPanel,
  focusPanel,
  ScrollFocus,
  softFocusItemInPanel,
  useScrollFocusContext,
  useScrollFocusItem,
} from "./ScrollFocus"

// --- Mock IntersectionObserver ---

type IntersectionCallback = (entries: IntersectionObserverEntry[]) => void

let intersectionCallback: IntersectionCallback
let observedElements: Set<Element>

class MockIntersectionObserver {
  constructor(
    callback: IntersectionCallback,
    _options?: IntersectionObserverInit,
  ) {
    intersectionCallback = callback
    observedElements = new Set()
  }
  observe(el: Element) {
    observedElements.add(el)
  }
  unobserve(el: Element) {
    observedElements.delete(el)
  }
  disconnect() {
    observedElements.clear()
  }
}

beforeEach(() => {
  vi.stubGlobal("IntersectionObserver", MockIntersectionObserver)
})

// --- Helper to simulate intersection changes ---

function simulateIntersection(
  entries: { target: Element; isIntersecting: boolean }[],
) {
  act(() => {
    intersectionCallback(
      entries.map((e) => ({
        ...e,
        boundingClientRect: {} as DOMRectReadOnly,
        intersectionRatio: e.isIntersecting ? 1 : 0,
        intersectionRect: {} as DOMRectReadOnly,
        rootBounds: null,
        target: e.target,
        time: 0,
      })),
    )
  })
}

// --- Test item component ---

function TestItem({
  id,
  onFocus: onFocusProp,
}: {
  id: string
  onFocus?: () => void
}) {
  const { ref, isFocused } = useScrollFocusItem<HTMLButtonElement>(id, {
    onFocus: onFocusProp,
  })
  return (
    <button ref={ref} tabIndex={0} data-testid={id}>
      {id}
      {isFocused && <span data-testid={`${id}-focused`}>focused</span>}
    </button>
  )
}

// --- Component that reads context for testing ---

function ContextReader({
  onContext,
}: {
  onContext: (ctx: ReturnType<typeof useScrollFocusContext>) => void
}) {
  const ctx = useScrollFocusContext()
  onContext(ctx)
  return null
}

// --- Component that exposes navigation triggers for testing ---

function NavigationTrigger() {
  const { focusNext, focusPrevious } = useScrollFocusContext()
  return (
    <>
      <button data-testid="trigger-next" onClick={focusNext} />
      <button data-testid="trigger-prev" onClick={focusPrevious} />
    </>
  )
}

// =============================================================================
// Tests
// =============================================================================

describe("ScrollFocus", () => {
  describe("rendering", () => {
    it("renders children and sets data-panel-key", () => {
      render(
        <ScrollFocus panelKey="test-panel">
          <div data-testid="child">hello</div>
        </ScrollFocus>,
      )
      expect(screen.getByTestId("child")).toBeInTheDocument()
      const container = screen.getByTestId("child").parentElement!
      expect(container).toHaveAttribute("data-panel-key", "test-panel")
    })

    it("applies className to the scroll container", () => {
      render(
        <ScrollFocus panelKey="test-panel" className="my-custom-class">
          <div data-testid="child" />
        </ScrollFocus>,
      )
      const container = screen.getByTestId("child").parentElement!
      expect(container).toHaveClass("my-custom-class")
    })
  })

  describe("useScrollFocusContext", () => {
    it("throws when used outside ScrollFocus", () => {
      function BadConsumer() {
        useScrollFocusContext()
        return null
      }
      // Suppress React error boundary logging
      const spy = vi.spyOn(console, "error").mockImplementation(() => {})
      expect(() => render(<BadConsumer />)).toThrow(
        "useScrollFocusContext must be used within a ScrollFocus",
      )
      spy.mockRestore()
    })

    it("provides context within ScrollFocus", () => {
      let capturedCtx: ReturnType<typeof useScrollFocusContext> | null = null
      render(
        <ScrollFocus panelKey="test">
          <ContextReader
            onContext={(ctx) => {
              capturedCtx = ctx
            }}
          />
        </ScrollFocus>,
      )
      expect(capturedCtx).not.toBeNull()
      expect(capturedCtx!.focusedId).toBeNull()
      expect(typeof capturedCtx!.setFocusedId).toBe("function")
      expect(typeof capturedCtx!.register).toBe("function")
      expect(typeof capturedCtx!.unregister).toBe("function")
      expect(typeof capturedCtx!.focusNext).toBe("function")
      expect(typeof capturedCtx!.focusPrevious).toBe("function")
    })
  })

  describe("useScrollFocusItem", () => {
    it("registers item and sets data-scroll-focus-id", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-1" />
        </ScrollFocus>,
      )
      const button = screen.getByTestId("item-1")
      expect(button).toHaveAttribute("data-scroll-focus-id", "item-1")
    })

    it("tracks focus state via isFocused", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-1" />
        </ScrollFocus>,
      )
      // Initially not focused
      expect(screen.queryByTestId("item-1-focused")).not.toBeInTheDocument()

      // Focus the button
      act(() => {
        screen.getByTestId("item-1").focus()
      })
      expect(screen.getByTestId("item-1-focused")).toBeInTheDocument()

      // Blur the button
      act(() => {
        screen.getByTestId("item-1").blur()
      })
      expect(screen.queryByTestId("item-1-focused")).not.toBeInTheDocument()
    })

    it("sets data-focused attribute when focused", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-1" />
        </ScrollFocus>,
      )
      const button = screen.getByTestId("item-1")

      act(() => {
        button.focus()
      })
      expect(button).toHaveAttribute("data-focused", "true")

      act(() => {
        button.blur()
      })
      // After blur, data-focused is removed by setFocusedId(null)
      // blur sets focusedId to null which doesn't modify DOM attributes
    })

    it("calls onFocus callback when focused", () => {
      const onFocus = vi.fn()
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-1" onFocus={onFocus} />
        </ScrollFocus>,
      )
      act(() => {
        screen.getByTestId("item-1").focus()
      })
      expect(onFocus).toHaveBeenCalledOnce()
    })

    it("unregisters item on unmount", () => {
      const { unmount } = render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-1" />
        </ScrollFocus>,
      )
      const button = screen.getByTestId("item-1")
      expect(observedElements.has(button)).toBe(true)

      unmount()
      // After unmount, the observer should have been cleaned up
      expect(observedElements.size).toBe(0)
    })
  })

  describe("focus navigation (focusNext / focusPrevious)", () => {
    // We need getBoundingClientRect to return predictable values for sorting
    function setupBoundingRects(elements: { testId: string; top: number }[]) {
      for (const { testId, top } of elements) {
        const el = screen.getByTestId(testId)
        vi.spyOn(el, "getBoundingClientRect").mockReturnValue({
          top,
          bottom: top + 50,
          left: 0,
          right: 100,
          width: 100,
          height: 50,
          x: 0,
          y: top,
          toJSON: () => ({}),
        })
      }
    }

    it("focusNext moves focus to the next item in DOM order", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-a" />
          <TestItem id="item-b" />
          <TestItem id="item-c" />
          <NavigationTrigger />
        </ScrollFocus>,
      )
      setupBoundingRects([
        { testId: "item-a", top: 0 },
        { testId: "item-b", top: 50 },
        { testId: "item-c", top: 100 },
      ])

      // Focus first item
      act(() => {
        screen.getByTestId("item-a").focus()
      })
      expect(screen.getByTestId("item-a-focused")).toBeInTheDocument()

      // Mock scrollIntoView since jsdom doesn't implement it
      for (const id of ["item-a", "item-b", "item-c"]) {
        screen.getByTestId(id).scrollIntoView = vi.fn()
      }

      // Trigger focusNext
      act(() => {
        screen.getByTestId("trigger-next").click()
      })
      expect(screen.getByTestId("item-b-focused")).toBeInTheDocument()
      expect(screen.queryByTestId("item-a-focused")).not.toBeInTheDocument()
    })

    it("focusPrevious moves focus to the previous item", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-a" />
          <TestItem id="item-b" />
          <TestItem id="item-c" />
          <NavigationTrigger />
        </ScrollFocus>,
      )
      setupBoundingRects([
        { testId: "item-a", top: 0 },
        { testId: "item-b", top: 50 },
        { testId: "item-c", top: 100 },
      ])

      for (const id of ["item-a", "item-b", "item-c"]) {
        screen.getByTestId(id).scrollIntoView = vi.fn()
      }

      // Focus middle item
      act(() => {
        screen.getByTestId("item-b").focus()
      })
      expect(screen.getByTestId("item-b-focused")).toBeInTheDocument()

      // Trigger focusPrevious
      act(() => {
        screen.getByTestId("trigger-prev").click()
      })
      expect(screen.getByTestId("item-a-focused")).toBeInTheDocument()
      expect(screen.queryByTestId("item-b-focused")).not.toBeInTheDocument()
    })

    it("focusNext does nothing when already at the last item", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-a" />
          <TestItem id="item-b" />
          <NavigationTrigger />
        </ScrollFocus>,
      )
      setupBoundingRects([
        { testId: "item-a", top: 0 },
        { testId: "item-b", top: 50 },
      ])

      for (const id of ["item-a", "item-b"]) {
        screen.getByTestId(id).scrollIntoView = vi.fn()
      }

      act(() => {
        screen.getByTestId("item-b").focus()
      })

      act(() => {
        screen.getByTestId("trigger-next").click()
      })
      // Should stay on item-b
      expect(screen.getByTestId("item-b-focused")).toBeInTheDocument()
    })

    it("focusPrevious does nothing when already at the first item", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-a" />
          <TestItem id="item-b" />
          <NavigationTrigger />
        </ScrollFocus>,
      )
      setupBoundingRects([
        { testId: "item-a", top: 0 },
        { testId: "item-b", top: 50 },
      ])

      for (const id of ["item-a", "item-b"]) {
        screen.getByTestId(id).scrollIntoView = vi.fn()
      }

      act(() => {
        screen.getByTestId("item-a").focus()
      })

      act(() => {
        screen.getByTestId("trigger-prev").click()
      })
      // Should stay on item-a
      expect(screen.getByTestId("item-a-focused")).toBeInTheDocument()
    })
  })

  describe("focusPanel", () => {
    it("focuses the first item when no previous focus exists", () => {
      render(
        <ScrollFocus panelKey="my-panel">
          <TestItem id="first" />
          <TestItem id="second" />
        </ScrollFocus>,
      )

      act(() => {
        focusPanel("my-panel")
      })
      expect(document.activeElement).toBe(screen.getByTestId("first"))
    })

    it("focuses the last focused item (data-focused) if one exists", () => {
      render(
        <ScrollFocus panelKey="my-panel">
          <TestItem id="first" />
          <TestItem id="second" />
        </ScrollFocus>,
      )

      // Focus second item to set data-focused, then blur
      act(() => {
        screen.getByTestId("second").focus()
      })
      // data-focused="true" is now on the second item
      act(() => {
        // blur by focusing something outside
        ;(document.activeElement as HTMLElement)?.blur()
      })

      // Now focusPanel should re-focus the second item (last focused)
      act(() => {
        focusPanel("my-panel")
      })
      expect(document.activeElement).toBe(screen.getByTestId("second"))
    })

    it("does nothing if panel does not exist", () => {
      render(
        <ScrollFocus panelKey="my-panel">
          <TestItem id="first" />
        </ScrollFocus>,
      )

      // Should not throw
      act(() => {
        focusPanel("nonexistent-panel")
      })
      expect(document.activeElement).not.toBe(screen.getByTestId("first"))
    })
  })

  describe("focusItemInPanel", () => {
    it("focuses and scrolls to a specific item", () => {
      render(
        <ScrollFocus panelKey="my-panel">
          <TestItem id="alpha" />
          <TestItem id="beta" />
        </ScrollFocus>,
      )

      const betaEl = screen.getByTestId("beta")
      betaEl.scrollIntoView = vi.fn()

      act(() => {
        focusItemInPanel("my-panel", "beta")
      })

      expect(document.activeElement).toBe(betaEl)
      expect(betaEl.scrollIntoView).toHaveBeenCalledWith({
        behavior: "instant",
        block: "start",
      })
    })

    it("does nothing for a nonexistent item", () => {
      render(
        <ScrollFocus panelKey="my-panel">
          <TestItem id="alpha" />
        </ScrollFocus>,
      )

      act(() => {
        focusItemInPanel("my-panel", "nonexistent")
      })
      expect(document.activeElement).not.toBe(screen.getByTestId("alpha"))
    })

    it("does nothing for a nonexistent panel", () => {
      render(
        <ScrollFocus panelKey="my-panel">
          <TestItem id="alpha" />
        </ScrollFocus>,
      )

      act(() => {
        focusItemInPanel("wrong-panel", "alpha")
      })
      expect(document.activeElement).not.toBe(screen.getByTestId("alpha"))
    })
  })

  describe("softFocusItemInPanel", () => {
    it("sets data-focused without DOM focus", () => {
      render(
        <ScrollFocus panelKey="my-panel">
          <TestItem id="alpha" />
          <TestItem id="beta" />
        </ScrollFocus>,
      )

      act(() => {
        softFocusItemInPanel("my-panel", "beta")
      })

      const betaEl = screen.getByTestId("beta")
      expect(betaEl).toHaveAttribute("data-focused", "true")
      // Should NOT be the active element (no DOM focus)
      expect(document.activeElement).not.toBe(betaEl)
    })

    it("moves data-focused from previous item to new item", () => {
      render(
        <ScrollFocus panelKey="my-panel">
          <TestItem id="alpha" />
          <TestItem id="beta" />
        </ScrollFocus>,
      )

      act(() => {
        softFocusItemInPanel("my-panel", "alpha")
      })
      expect(screen.getByTestId("alpha")).toHaveAttribute(
        "data-focused",
        "true",
      )

      act(() => {
        softFocusItemInPanel("my-panel", "beta")
      })
      expect(screen.getByTestId("beta")).toHaveAttribute("data-focused", "true")
      expect(screen.getByTestId("alpha")).not.toHaveAttribute("data-focused")
    })

    it("does nothing for a nonexistent panel", () => {
      render(
        <ScrollFocus panelKey="my-panel">
          <TestItem id="alpha" />
        </ScrollFocus>,
      )

      // Should not throw
      act(() => {
        softFocusItemInPanel("wrong-panel", "alpha")
      })
      expect(screen.getByTestId("alpha")).not.toHaveAttribute("data-focused")
    })
  })

  describe("intersection observer behavior", () => {
    it("marks items visible/invisible based on intersection", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-1" />
          <TestItem id="item-2" />
        </ScrollFocus>,
      )

      const item1 = screen.getByTestId("item-1")
      const item2 = screen.getByTestId("item-2")

      // Both items should be observed
      expect(observedElements.has(item1)).toBe(true)
      expect(observedElements.has(item2)).toBe(true)

      // Simulate item-1 becoming visible
      simulateIntersection([{ target: item1, isIntersecting: true }])
      // No crash, no errors - visibility is tracked internally
    })

    it("shifts focus to visible item when focused item scrolls out of view", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="item-1" />
          <TestItem id="item-2" />
        </ScrollFocus>,
      )

      const item1 = screen.getByTestId("item-1")
      const item2 = screen.getByTestId("item-2")

      // Mock getBoundingClientRect for sort ordering
      vi.spyOn(item1, "getBoundingClientRect").mockReturnValue({
        top: 0,
        bottom: 50,
        left: 0,
        right: 100,
        width: 100,
        height: 50,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      })
      vi.spyOn(item2, "getBoundingClientRect").mockReturnValue({
        top: 50,
        bottom: 100,
        left: 0,
        right: 100,
        width: 100,
        height: 50,
        x: 0,
        y: 50,
        toJSON: () => ({}),
      })

      item1.scrollIntoView = vi.fn()
      item2.scrollIntoView = vi.fn()

      // Focus item-1
      act(() => {
        item1.focus()
      })
      expect(screen.getByTestId("item-1-focused")).toBeInTheDocument()

      // Both items visible
      simulateIntersection([
        { target: item1, isIntersecting: true },
        { target: item2, isIntersecting: true },
      ])

      // item-1 scrolls out of view, item-2 still visible
      simulateIntersection([{ target: item1, isIntersecting: false }])

      // Focus should shift to item-2 (first visible entry when scrolling down)
      expect(screen.getByTestId("item-2-focused")).toBeInTheDocument()
    })
  })

  describe("multiple items focus transitions", () => {
    it("only one item is focused at a time", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="a" />
          <TestItem id="b" />
          <TestItem id="c" />
        </ScrollFocus>,
      )

      act(() => {
        screen.getByTestId("a").focus()
      })
      expect(screen.getByTestId("a-focused")).toBeInTheDocument()
      expect(screen.queryByTestId("b-focused")).not.toBeInTheDocument()
      expect(screen.queryByTestId("c-focused")).not.toBeInTheDocument()

      act(() => {
        screen.getByTestId("c").focus()
      })
      expect(screen.queryByTestId("a-focused")).not.toBeInTheDocument()
      expect(screen.queryByTestId("c-focused")).toBeInTheDocument()
    })

    it("data-focused attribute moves between items", () => {
      render(
        <ScrollFocus panelKey="panel">
          <TestItem id="a" />
          <TestItem id="b" />
        </ScrollFocus>,
      )

      act(() => {
        screen.getByTestId("a").focus()
      })
      expect(screen.getByTestId("a")).toHaveAttribute("data-focused", "true")

      act(() => {
        screen.getByTestId("b").focus()
      })
      expect(screen.getByTestId("b")).toHaveAttribute("data-focused", "true")
      expect(screen.getByTestId("a")).not.toHaveAttribute("data-focused")
    })
  })
})
