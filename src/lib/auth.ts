import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { LazyStore } from "@tauri-apps/plugin-store"

const store = new LazyStore("auth.json")
const TOKEN_KEY = "github_token"

/**
 * Get stored token from plugin-store.
 */
export async function getStoredToken(): Promise<string | null> {
  return (await store.get<string>(TOKEN_KEY)) ?? null
}

/**
 * Set up listener for token from Rust OAuth flow.
 * Returns unlisten function.
 */
export function setupAuthListener(
  onToken: (token: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("auth-token", async (event) => {
    const token = event.payload

    // Store token for persistence across app restarts
    await store.set(TOKEN_KEY, token)
    await store.save()

    // Notify caller
    onToken(token)
  })
}
