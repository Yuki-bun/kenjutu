import { LazyStore } from "@tauri-apps/plugin-store"

const store = new LazyStore("repos.json")

/**
 * Get local path for a GitHub repository.
 */
export async function getLocalPath(ghRepoId: string): Promise<string | null> {
  return (await store.get<string>(ghRepoId)) ?? null
}

export async function getLocalRepoDirs(): Promise<string[]> {
  return store.values()
}

/**
 * Set local path for a GitHub repository.
 */
export async function setLocalPath(
  ghRepoId: string,
  localPath: string,
): Promise<void> {
  await store.set(ghRepoId, localPath)
  await store.save()
}
