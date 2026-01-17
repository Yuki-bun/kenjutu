import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from "react"
import { Octokit } from "@octokit/rest"
import { getStoredToken, setupAuthListener } from "@/lib/auth"

interface GithubContextValue {
  octokit: Octokit | null
  isAuthenticated: boolean
}

const GithubContext = createContext<GithubContextValue | null>(null)

export function GithubProvider({ children }: { children: ReactNode }) {
  const [octokit, setOctokit] = useState<Octokit | null>(null)

  const initializeOctokit = useCallback((token: string) => {
    setOctokit(new Octokit({ auth: token }))
  }, [])

  useEffect(() => {
    // Load stored token on mount
    getStoredToken().then((token) => {
      if (token) {
        initializeOctokit(token)
      }
    })

    // Listen for new tokens from OAuth flow
    const unlistenPromise = setupAuthListener((token) => {
      initializeOctokit(token)
    })

    return () => {
      unlistenPromise.then((unlisten) => unlisten())
    }
  }, [initializeOctokit])

  return (
    <GithubContext.Provider
      value={{
        octokit,
        isAuthenticated: octokit !== null,
      }}
    >
      {children}
    </GithubContext.Provider>
  )
}

export function useGithub(): GithubContextValue {
  const context = useContext(GithubContext)
  if (!context) {
    throw new Error("useGithub must be used within a GithubProvider")
  }
  return context
}

export function useOctokit(): Octokit {
  const { octokit } = useGithub()
  if (!octokit) {
    throw new Error("Octokit not initialized. User must be authenticated.")
  }
  return octokit
}
