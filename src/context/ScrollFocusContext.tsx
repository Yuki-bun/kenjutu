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
  setFocusedId: (id: string) => void
  register: (id: string, ref: RefObject<HTMLElement | null>) => void
  unregister: (id: string) => void
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
  const { focusedId, setFocusedId, register, unregister } = useScrollFocus({
    scrollContainerRef,
  })

  return (
    <ScrollFocusContext.Provider
      value={{
        focusedId,
        setFocusedId,
        register,
        unregister,
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

  const handleFocus = () => setFocusedId(id)

  return { ref, isFocused, handleFocus }
}
