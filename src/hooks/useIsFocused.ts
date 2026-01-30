import { useEffect, useRef, useState } from "react"

export function useIsFocused<T extends HTMLDivElement>() {
  const [isFocused, setIsFocused] = useState(false)
  const ref = useRef<T>(null)

  useEffect(() => {
    const element = ref.current
    if (!element) return

    const onFocus = () => setIsFocused(true)
    const onBlur = () => setIsFocused(false)

    element.addEventListener("focusin", onFocus)
    element.addEventListener("focusout", onBlur)

    return () => {
      element.removeEventListener("focusin", onFocus)
      element.removeEventListener("focusout", onBlur)
    }
  }, [])

  return [ref, isFocused] as const
}
