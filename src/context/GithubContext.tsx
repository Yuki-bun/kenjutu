import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from "react"
import { Octokit } from "@octokit/rest"
import { getStoredToken, setupAuthListener, clearStoredToken } from "@/lib/auth"

interface GithubContextValue {
  octokit: Octokit | null
  isAuthenticated: boolean
}

const GithubContext = createContext<GithubContextValue | null>(null)

export function GithubProvider({ children }: { children: ReactNode }) {
  const [octokit, setOctokit] = useState<Octokit | null>(null)

  const initializeOctokit = useCallback((token: string) => {
    const kit = new Octokit({ auth: token })

    kit.hook.error("request", (error) => {
      if (isBadCredentialError(error)) {
        setOctokit(null)
        clearStoredToken()
      }
      throw error
    })

    setOctokit(kit)
  }, [])

  useEffect(() => {
    getStoredToken().then((token) => {
      if (token) {
        initializeOctokit(token)
      }
    })

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

function isBadCredentialError(error: unknown) {
  return (
    error &&
    typeof error === "object" &&
    "status" in error &&
    error.status === 401
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
