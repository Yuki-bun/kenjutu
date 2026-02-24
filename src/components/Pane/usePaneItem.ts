import { useCallback, useEffect, useRef } from "react"

import { usePaneContext } from "./Pane"

type UsePaneItemOptions = {
  onBlur?: () => void
}

export function usePaneItem<T extends HTMLElement = HTMLElement>(
  id: string,
  options?: UsePaneItemOptions,
) {
  const ref = useRef<T>(null)
  const { focusedId, setFocusedId, register, unregister } = usePaneContext()
  const onBlurRef = useRef(options?.onBlur)
  onBlurRef.current = options?.onBlur

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
    }
    const handleBlur = () => {
      setFocusedId(null)
      onBlurRef.current?.()
    }

    element.addEventListener("focus", handleFocus)
    element.addEventListener("blur", handleBlur)
    return () => {
      element.removeEventListener("focus", handleFocus)
      element.removeEventListener("blur", handleBlur)
    }
  }, [id, setFocusedId])

  const isFocused = focusedId === id

  const scrollIntoView = useCallback(() => {
    const element = ref.current
    if (element) {
      element.scrollIntoView(true)
    }
  }, [])

  return { ref, isFocused, scrollIntoView }
}
