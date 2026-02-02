import {
  createContext,
  ReactNode,
  RefObject,
  useContext,
  useEffect,
  useRef,
} from "react"

import { useScrollFocus } from "@/hooks/useScrollFocus"

interface ScrollFocusContextValue {
  focusedId: string | null
  setFocusedId: (id: string | null) => void
  register: (id: string, ref: RefObject<HTMLElement | null>) => void
  unregister: (id: string) => void
  focusNext: () => void
  focusPrevious: () => void
}

const ScrollFocusContext = createContext<ScrollFocusContextValue | null>(null)

type ScrollFocusProviderProps = {
  children: ReactNode
  scrollContainerRef?: RefObject<HTMLElement | null>
}

export function ScrollFocusProvider({
  children,
  scrollContainerRef,
}: ScrollFocusProviderProps) {
  const {
    focusedId,
    setFocusedId,
    register,
    unregister,
    focusNext,
    focusPrevious,
  } = useScrollFocus({
    scrollContainerRef,
  })

  return (
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
  )
}

export function useScrollFocusContext() {
  const context = useContext(ScrollFocusContext)
  if (!context) {
    throw new Error(
      "useScrollFocusContext must be used within a ScrollFocusProvider",
    )
  }
  return context
}

export function useScrollFocusItem<T extends HTMLElement>(id: string) {
  const ref = useRef<T>(null)
  const { focusedId, setFocusedId, register, unregister } =
    useScrollFocusContext()

  const isFocused = focusedId === id

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

    const handleFocus = () => setFocusedId(id)
    const handleBlur = () => setFocusedId(null)

    element.addEventListener("focus", handleFocus)
    element.addEventListener("blur", handleBlur)
    return () => {
      element.removeEventListener("focus", handleFocus)
      element.removeEventListener("blur", handleBlur)
    }
  }, [id, setFocusedId])

  return { ref, isFocused }
}
