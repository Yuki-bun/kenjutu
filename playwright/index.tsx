import "../src/index.css"
import {
  focusPanel,
  focusItemInPanel,
  softFocusItemInPanel,
} from "../src/components/ScrollFocus"

// Expose helper functions on window for use in page.evaluate()
declare global {
  interface Window {
    scrollFocusHelpers: {
      focusPanel: typeof focusPanel
      focusItemInPanel: typeof focusItemInPanel
      softFocusItemInPanel: typeof softFocusItemInPanel
    }
  }
}

window.scrollFocusHelpers = {
  focusPanel,
  focusItemInPanel,
  softFocusItemInPanel,
}
