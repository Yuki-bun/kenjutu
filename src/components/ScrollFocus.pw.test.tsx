import { expect, test } from "@playwright/experimental-ct-react"
import type { Page } from "@playwright/test"

import {
  DynamicTestList,
  EmptyTestList,
  TestList,
} from "./ScrollFocus.test-utils"

// Declare the window type for scrollFocusHelpers
declare global {
  interface Window {
    scrollFocusHelpers: {
      focusPanel: (panelKey: string) => void
      focusItemInPanel: (panelKey: string, itemId: string) => void
      softFocusItemInPanel: (panelKey: string, itemId: string) => void
    }
  }
}

// =============================================================================
// Helper Functions for Tests
// =============================================================================

async function callFocusPanel(page: Page, panelKey: string) {
  await page.evaluate((key: string) => {
    window.scrollFocusHelpers.focusPanel(key)
  }, panelKey)
}

async function callFocusItemInPanel(
  page: Page,
  panelKey: string,
  itemId: string,
) {
  await page.evaluate(
    ({ key, id }: { key: string; id: string }) => {
      window.scrollFocusHelpers.focusItemInPanel(key, id)
    },
    { key: panelKey, id: itemId },
  )
}

async function callSoftFocusItemInPanel(
  page: Page,
  panelKey: string,
  itemId: string,
) {
  await page.evaluate(
    ({ key, id }: { key: string; id: string }) => {
      window.scrollFocusHelpers.softFocusItemInPanel(key, id)
    },
    { key: panelKey, id: itemId },
  )
}

async function getScrollTop(page: Page): Promise<number> {
  return page.evaluate(() => {
    const container = document.querySelector('[data-panel-key="test-panel"]')
    return container?.scrollTop ?? 0
  })
}

async function scrollTo(page: Page, scrollTop: number) {
  await page.evaluate((top: number) => {
    const container = document.querySelector('[data-panel-key="test-panel"]')
    container?.scrollTo({ top, behavior: "instant" })
  }, scrollTop)
  // Wait for IntersectionObserver callbacks
  await page.waitForTimeout(100)
}

// =============================================================================
// 1. Keyboard Navigation Tests
// =============================================================================

test.describe("1. Keyboard Navigation", () => {
  test("j key moves focus to next item", async ({ mount, page }) => {
    await mount(<TestList />)

    // Focus item-0
    await page.getByTestId("item-0").focus()
    await expect(page.getByTestId("item-0")).toBeFocused()

    // Press j
    await page.keyboard.press("j")

    // Verify item-1 is focused
    await expect(page.getByTestId("item-1")).toBeFocused()
    await expect(page.getByTestId("item-1")).toHaveAttribute(
      "data-focused",
      "true",
    )
  })

  test("k key moves focus to previous item", async ({ mount, page }) => {
    await mount(<TestList />)

    // Focus item-2
    await page.getByTestId("item-2").focus()
    await expect(page.getByTestId("item-2")).toBeFocused()

    // Press k
    await page.keyboard.press("k")

    // Verify item-1 is focused
    await expect(page.getByTestId("item-1")).toBeFocused()
    await expect(page.getByTestId("item-1")).toHaveAttribute(
      "data-focused",
      "true",
    )
  })

  test("j key does nothing at last item", async ({ mount, page }) => {
    await mount(<TestList itemCount={5} />)

    // Focus last item (item-4)
    await page.getByTestId("item-4").focus()
    await expect(page.getByTestId("item-4")).toBeFocused()

    // Press j
    await page.keyboard.press("j")

    // Verify focus stays on item-4
    await expect(page.getByTestId("item-4")).toBeFocused()
  })

  test("k key does nothing at first item", async ({ mount, page }) => {
    await mount(<TestList />)

    // Focus item-0
    await page.getByTestId("item-0").focus()
    await expect(page.getByTestId("item-0")).toBeFocused()

    // Press k
    await page.keyboard.press("k")

    // Verify focus stays on item-0
    await expect(page.getByTestId("item-0")).toBeFocused()
  })

  test("shift+j scrolls down without changing focus", async ({
    mount,
    page,
  }) => {
    await mount(<TestList itemCount={20} itemHeight={60} />)

    // Focus item-2 (not item-0, so it stays visible after a small scroll)
    await page.getByTestId("item-2").focus()
    await expect(page.getByTestId("item-2")).toBeFocused()

    const initialScrollTop = await getScrollTop(page)

    // Press shift+j - scrolls by 100px
    await page.keyboard.press("Shift+j")
    await page.waitForTimeout(100)

    // Verify scrollTop increased but item-2 still focused
    const newScrollTop = await getScrollTop(page)
    expect(newScrollTop).toBeGreaterThan(initialScrollTop)
    await expect(page.getByTestId("item-2")).toBeFocused()
  })

  test("shift+k scrolls up without changing focus", async ({ mount, page }) => {
    await mount(<TestList itemCount={20} />)

    // Scroll down first
    await scrollTo(page, 300)

    // Focus item-5 (should be visible after scrolling)
    await page.getByTestId("item-5").focus()
    await expect(page.getByTestId("item-5")).toBeFocused()

    const initialScrollTop = await getScrollTop(page)
    expect(initialScrollTop).toBeGreaterThan(0) // sanity check

    // Press shift+k
    await page.keyboard.press("Shift+k")
    await page.waitForTimeout(100)

    // Verify scrollTop decreased but item-5 still focused
    const newScrollTop = await getScrollTop(page)
    expect(newScrollTop).toBeLessThan(initialScrollTop)
    await expect(page.getByTestId("item-5")).toBeFocused()
  })

  test("keyboard navigation disabled when no item focused", async ({
    mount,
    page,
  }) => {
    await mount(<TestList />)

    // Don't focus any item - just verify no item is focused initially
    const focusedItem = await page.evaluate(() => {
      return document.activeElement?.getAttribute("data-testid") ?? ""
    })
    expect(focusedItem).not.toMatch(/^item-\d+$/)

    // Press j - should not crash and should not focus anything
    await page.keyboard.press("j")

    // Verify no item became focused
    const focusedItemAfter = await page.evaluate(() => {
      return document.activeElement?.getAttribute("data-testid") ?? ""
    })
    expect(focusedItemAfter).not.toMatch(/^item-\d+$/)
  })
})

// =============================================================================
// 2. Scroll-Based Focus Transfer Tests
// =============================================================================

test.describe("2. Scroll-Based Focus Transfer", () => {
  test("focus transfers to first visible item when scrolling down", async ({
    mount,
    page,
  }) => {
    await mount(<TestList itemCount={20} itemHeight={80} />)

    // Focus item-0
    await page.getByTestId("item-0").focus()
    await expect(page.getByTestId("item-0")).toBeFocused()

    // Scroll down until item-0 is out of view
    await scrollTo(page, 400)

    // Wait for IntersectionObserver and focus transfer
    await page.waitForTimeout(200)

    // Verify focus moved away from item-0
    await expect(page.getByTestId("item-0")).not.toBeFocused()

    // Verify some visible item is now focused
    const focusedId = await page.evaluate(() => {
      return document.activeElement?.getAttribute("data-testid")
    })
    expect(focusedId).toMatch(/^item-\d+$/)
  })

  test("focus transfers to last visible item when scrolling up", async ({
    mount,
    page,
  }) => {
    await mount(<TestList itemCount={20} itemHeight={80} />)

    // Scroll to bottom first
    await scrollTo(page, 1300)

    // Focus a visible item near the bottom (item-19)
    await page.getByTestId("item-19").focus()
    await expect(page.getByTestId("item-19")).toBeFocused()

    // Scroll up until item-19 is out of view
    await scrollTo(page, 0)

    // Wait for IntersectionObserver and focus transfer
    await page.waitForTimeout(200)

    // Verify focus moved away from item-19
    await expect(page.getByTestId("item-19")).not.toBeFocused()

    // Verify some visible item is now focused
    const focusedId = await page.evaluate(() => {
      return document.activeElement?.getAttribute("data-testid")
    })
    expect(focusedId).toMatch(/^item-\d+$/)
  })

  test("focus stays if item remains visible", async ({ mount, page }) => {
    await mount(<TestList itemCount={20} itemHeight={60} />)

    // Focus item-2
    await page.getByTestId("item-2").focus()
    await expect(page.getByTestId("item-2")).toBeFocused()

    // Scroll slightly (item-2 should still be visible)
    await scrollTo(page, 30)

    // Verify focus unchanged
    await expect(page.getByTestId("item-2")).toBeFocused()
    await expect(page.getByTestId("item-2")).toHaveAttribute(
      "data-focused",
      "true",
    )
  })

  test("rapid scrolling settles on correct visible item", async ({
    mount,
    page,
  }) => {
    await mount(<TestList itemCount={30} itemHeight={60} />)

    // Focus item-0
    await page.getByTestId("item-0").focus()

    // Rapid scroll to bottom
    await scrollTo(page, 500)
    await scrollTo(page, 1000)
    await scrollTo(page, 1500)

    // Wait for things to settle
    await page.waitForTimeout(300)

    // Verify a visible item is focused
    const focusedId = await page.evaluate(() => {
      return document.activeElement?.getAttribute("data-testid")
    })
    expect(focusedId).toMatch(/^item-\d+$/)

    // Verify the focused item is actually visible in the container
    const isVisible = await page.evaluate(() => {
      const container = document.querySelector('[data-panel-key="test-panel"]')
      const focused = document.activeElement
      if (!container || !focused) return false

      const containerRect = container.getBoundingClientRect()
      const focusedRect = focused.getBoundingClientRect()

      return (
        focusedRect.top >= containerRect.top &&
        focusedRect.bottom <= containerRect.bottom
      )
    })
    expect(isVisible).toBe(true)
  })
})

// =============================================================================
// 3. Panel Helper Functions Tests
// =============================================================================

test.describe("3. Panel Helper Functions", () => {
  test("focusPanel focuses first item when no previous focus", async ({
    mount,
    page,
  }) => {
    await mount(<TestList />)

    // No item focused initially
    await expect(page.getByTestId("item-0")).not.toBeFocused()

    // Call focusPanel
    await callFocusPanel(page, "test-panel")

    // Verify item-0 is focused
    await expect(page.getByTestId("item-0")).toBeFocused()
  })

  test("focusPanel re-focuses last focused item", async ({ mount, page }) => {
    await mount(<TestList />)

    // Focus item-3
    await page.getByTestId("item-3").focus()
    await expect(page.getByTestId("item-3")).toBeFocused()

    // Blur it by focusing elsewhere
    await page.evaluate(() => {
      ;(document.activeElement as HTMLElement)?.blur()
    })
    await expect(page.getByTestId("item-3")).not.toBeFocused()

    // Call focusPanel
    await callFocusPanel(page, "test-panel")

    // Verify item-3 is focused again
    await expect(page.getByTestId("item-3")).toBeFocused()
  })

  test("focusPanel does nothing for nonexistent panel", async ({
    mount,
    page,
  }) => {
    await mount(<TestList />)

    // Focus item-0 first
    await page.getByTestId("item-0").focus()

    // Call focusPanel with wrong key - should not throw
    await callFocusPanel(page, "wrong-key")

    // Verify no crash and item-0 still focused
    await expect(page.getByTestId("item-0")).toBeFocused()
  })

  test("focusItemInPanel focuses and scrolls to specific item", async ({
    mount,
    page,
  }) => {
    await mount(<TestList itemCount={20} />)

    // item-15 should not be visible initially
    const initialScrollTop = await getScrollTop(page)
    expect(initialScrollTop).toBe(0)

    // Call focusItemInPanel
    await callFocusItemInPanel(page, "test-panel", "item-15")
    await page.waitForTimeout(150)

    // Verify scroll happened and item is focused
    const afterScroll = await page.evaluate(() => {
      const container = document.querySelector('[data-panel-key="test-panel"]')
      const item15 = document.querySelector('[data-testid="item-15"]')
      return {
        scrollTop: container?.scrollTop ?? 0,
        isFocused: document.activeElement === item15,
        isInView: (() => {
          if (!container || !item15) return false
          const cRect = container.getBoundingClientRect()
          const iRect = item15.getBoundingClientRect()
          return iRect.top >= cRect.top && iRect.bottom <= cRect.bottom
        })(),
      }
    })

    expect(afterScroll.scrollTop).toBeGreaterThan(0)
    expect(afterScroll.isInView).toBe(true)
  })

  test("focusItemInPanel does nothing for nonexistent item", async ({
    mount,
    page,
  }) => {
    await mount(<TestList />)

    // Focus item-0 first
    await page.getByTestId("item-0").focus()

    // Call focusItemInPanel with nonexistent item - should not throw
    await callFocusItemInPanel(page, "test-panel", "nonexistent")

    // Verify no crash and item-0 still focused
    await expect(page.getByTestId("item-0")).toBeFocused()
  })

  test("softFocusItemInPanel sets data-focused without DOM focus", async ({
    mount,
    page,
  }) => {
    await mount(<TestList />)

    // Call softFocusItemInPanel
    await callSoftFocusItemInPanel(page, "test-panel", "item-5")

    // Verify data-focused is set
    await expect(page.getByTestId("item-5")).toHaveAttribute(
      "data-focused",
      "true",
    )

    // Verify DOM focus is NOT on item-5
    const activeTestId = await page.evaluate(() => {
      return document.activeElement?.getAttribute("data-testid")
    })
    expect(activeTestId).not.toBe("item-5")
  })

  test("softFocusItemInPanel moves data-focused between items", async ({
    mount,
    page,
  }) => {
    await mount(<TestList />)

    // Soft focus item-3
    await callSoftFocusItemInPanel(page, "test-panel", "item-3")
    await expect(page.getByTestId("item-3")).toHaveAttribute(
      "data-focused",
      "true",
    )

    // Soft focus item-7
    await callSoftFocusItemInPanel(page, "test-panel", "item-7")

    // Verify item-7 has data-focused
    await expect(page.getByTestId("item-7")).toHaveAttribute(
      "data-focused",
      "true",
    )

    // Verify item-3 no longer has data-focused
    await expect(page.getByTestId("item-3")).not.toHaveAttribute("data-focused")
  })
})

// =============================================================================
// 4. Focus State & Attributes Tests
// =============================================================================

test.describe("4. Focus State & Attributes", () => {
  test("focused item has data-focused attribute", async ({ mount, page }) => {
    await mount(<TestList />)

    // Focus item-2
    await page.getByTestId("item-2").focus()

    // Verify data-focused="true"
    await expect(page.getByTestId("item-2")).toHaveAttribute(
      "data-focused",
      "true",
    )
  })

  test("data-focused moves when focus changes", async ({ mount, page }) => {
    await mount(<TestList />)

    // Focus item-1
    await page.getByTestId("item-1").focus()
    await expect(page.getByTestId("item-1")).toHaveAttribute(
      "data-focused",
      "true",
    )

    // Focus item-3
    await page.getByTestId("item-3").focus()

    // Verify item-3 has data-focused
    await expect(page.getByTestId("item-3")).toHaveAttribute(
      "data-focused",
      "true",
    )

    // Verify item-1 no longer has data-focused
    await expect(page.getByTestId("item-1")).not.toHaveAttribute("data-focused")
  })

  test("items have data-scroll-focus-id attribute", async ({ mount, page }) => {
    await mount(<TestList itemCount={5} />)

    // Verify each item has correct data-scroll-focus-id
    for (let i = 0; i < 5; i++) {
      await expect(page.getByTestId(`item-${i}`)).toHaveAttribute(
        "data-scroll-focus-id",
        `item-${i}`,
      )
    }
  })

  test("isFocused state updates correctly", async ({ mount, page }) => {
    await mount(<TestList />)

    // Focus item-2
    await page.getByTestId("item-2").focus()

    // Verify the focused indicator (✓) is rendered
    await expect(page.getByTestId("item-2-focused-indicator")).toBeVisible()

    // Focus item-3
    await page.getByTestId("item-3").focus()

    // Verify item-2's indicator is gone
    await expect(page.getByTestId("item-2-focused-indicator")).not.toBeVisible()

    // Verify item-3's indicator is visible
    await expect(page.getByTestId("item-3-focused-indicator")).toBeVisible()
  })
})

// =============================================================================
// 5. Edge Cases Tests
// =============================================================================

test.describe("5. Edge Cases", () => {
  test("empty panel handles focusPanel gracefully", async ({ mount, page }) => {
    await mount(<EmptyTestList />)

    // Call focusPanel - should not throw
    await callFocusPanel(page, "test-panel")

    // Verify no crash
    const container = page.locator('[data-panel-key="test-panel"]')
    await expect(container).toBeVisible()
  })

  test("single item navigation stays on that item", async ({ mount, page }) => {
    await mount(<TestList itemCount={1} />)

    // Focus the only item
    await page.getByTestId("item-0").focus()
    await expect(page.getByTestId("item-0")).toBeFocused()

    // Press j - should stay on item-0
    await page.keyboard.press("j")
    await expect(page.getByTestId("item-0")).toBeFocused()

    // Press k - should stay on item-0
    await page.keyboard.press("k")
    await expect(page.getByTestId("item-0")).toBeFocused()
  })

  test("dynamically added items are navigable", async ({ mount, page }) => {
    await mount(<DynamicTestList />)

    // Initially 3 items, focus item-0
    await page.getByTestId("item-0").focus()
    await expect(page.getByTestId("item-0")).toBeFocused()

    // Navigate through initial items
    await page.keyboard.press("j")
    await expect(page.getByTestId("item-1")).toBeFocused()
    await page.keyboard.press("j")
    await expect(page.getByTestId("item-2")).toBeFocused()

    // Can't go further
    await page.keyboard.press("j")
    await expect(page.getByTestId("item-2")).toBeFocused()

    // Add 2 more items (clicking button will take focus)
    await page.getByTestId("add-items").click()

    // Wait for re-render and new items to register
    await page.waitForTimeout(200)

    // Re-focus item-2 since button click took focus
    await page.getByTestId("item-2").focus()
    await expect(page.getByTestId("item-2")).toBeFocused()

    // Now we can navigate to item-3 and item-4
    await page.keyboard.press("j")
    await expect(page.getByTestId("item-3")).toBeFocused()

    await page.keyboard.press("j")
    await expect(page.getByTestId("item-4")).toBeFocused()
  })
})
