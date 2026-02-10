import * as Collapsible from "@radix-ui/react-collapsible"
import { useQuery } from "@tanstack/react-query"
import { createFileRoute } from "@tanstack/react-router"
import { ChevronDown, ChevronLeft, ChevronRight, ChevronUp } from "lucide-react"
import { useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"
import { toast } from "sonner"

import {
  CommitDiffSection,
  FILE_TREE_PANEL_KEY,
  FileTree,
} from "@/components/diff"
import { ErrorDisplay } from "@/components/error"
import { focusPanel, ScrollFocus } from "@/components/ScrollFocus"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { useGithub } from "@/context/GithubContext"
import { getLocalPath } from "@/lib/repos"

import { PR_COMMIT_LIST_PANEL_KEY, PRCommitList } from "./-PRCommitList"
import { useMergePullRequest } from "./-useMergePullRequest"
import { usePullRequest } from "./-usePullRequest"

export const Route = createFileRoute("/pulls/$owner/$repo/$number")({
  component: RouteComponent,
  validateSearch: (search: Record<string, unknown>) => {
    const repoId = search.repoId
    if (typeof repoId !== "string") {
      throw new Error("Pass repoId")
    }
    return { repoId }
  },
})

type CommitSelection =
  | {
      type: "change-id"
      changeId: string
    }
  | {
      type: "commit-id"
      commitId: string
    }

const DIFF_VIEW_PANEL_KEY = "diff-view"

function RouteComponent() {
  const { number, owner, repo } = Route.useParams()
  const { repoId } = Route.useSearch()
  const { isAuthenticated } = useGithub()
  const [commitSelection, setCommitSelection] =
    useState<CommitSelection | null>(null)
  const [isSidebarOpen, setIsSidebarOpen] = useState(true)
  const [isDescriptionOpen, setIsDescriptionOpen] = useState(true)

  // Fetch local repo path from Tauri Store
  const { data: localDir } = useQuery({
    queryKey: ["localRepoPath", repoId],
    queryFn: () => getLocalPath(repoId),
  })

  const { data, error, isLoading, refetch } = usePullRequest(
    localDir ?? null,
    owner,
    repo,
    Number(number),
  )

  const selectedCommit = data?.commits.find((commit) => {
    switch (commitSelection?.type) {
      case "change-id":
        return commit.changeId === commitSelection.changeId
      case "commit-id":
        return commit.sha === commitSelection.commitId
      case undefined:
        return false
    }
  })

  const mergeMutation = useMergePullRequest()

  const handleMerge = () => {
    mergeMutation.mutate(
      {
        owner,
        repo,
        pullNumber: Number(number),
      },
      {
        onSuccess: (result) => {
          toast.success("Pull request merged successfully!", {
            description: `SHA: ${result.sha}`,
            position: "top-center",
            duration: 5000,
          })
          refetch()
        },
        onError: (err) => {
          const message =
            err instanceof Error
              ? err.message
              : "Failed to merge pull request. Please try again."
          toast.error("Merge failed", {
            description: message,
            position: "top-center",
            duration: 7000,
          })
        },
      },
    )
  }

  // Keyboard shortcuts
  useHotkeys(
    "1",
    () => {
      if (isSidebarOpen) {
        focusPanel(PR_COMMIT_LIST_PANEL_KEY)
      } else {
        setIsSidebarOpen(true)
        setTimeout(() => focusPanel(PR_COMMIT_LIST_PANEL_KEY), 10)
      }
    },
    [isSidebarOpen],
  )

  useHotkeys(
    "2",
    () => {
      if (isSidebarOpen) {
        focusPanel(FILE_TREE_PANEL_KEY)
      } else {
        setIsSidebarOpen(true)
        setTimeout(() => focusPanel(FILE_TREE_PANEL_KEY), 10)
      }
    },
    [isSidebarOpen],
  )

  useHotkeys("3", () => focusPanel(DIFF_VIEW_PANEL_KEY))

  useHotkeys("meta+b", () => setIsSidebarOpen((open) => !open))

  // Full-width loading/error states before rendering layout
  if (isLoading) {
    return (
      <main className="h-full w-full p-4">
        <p className="text-muted-foreground">Loading pull request...</p>
      </main>
    )
  }

  if (error) {
    return (
      <main className="h-full w-full p-4">
        {error instanceof Error ? (
          error.message
        ) : (
          <ErrorDisplay error={error} />
        )}
      </main>
    )
  }

  return (
    <main className="flex h-full w-full">
      {/* Left: Collapsible Sidebar */}
      <Collapsible.Root
        open={isSidebarOpen}
        onOpenChange={setIsSidebarOpen}
        className="flex shrink-0 h-full"
      >
        <Collapsible.Content
          forceMount
          className="w-96 border-r overflow-y-auto data-[state=closed]:hidden"
        >
          {/* PR title + branches (compact) */}
          {data && (
            <div className="p-3 border-b">
              <h2 className="text-sm font-semibold truncate" title={data.title}>
                #{number} {data.title}
              </h2>
              <p className="text-xs text-muted-foreground">
                {data.base.ref} &larr; {data.head.ref}
              </p>
            </div>
          )}

          {/* Commit list */}
          {data && localDir && (
            <div className="border-b">
              <PRCommitList
                localDir={localDir}
                commits={data.commits}
                selectedCommitSha={selectedCommit?.sha ?? null}
                onSelectCommit={(commit) =>
                  setCommitSelection(
                    commit.changeId
                      ? { type: "change-id", changeId: commit.changeId }
                      : { type: "commit-id", commitId: commit.sha },
                  )
                }
              />
            </div>
          )}

          {/* File tree */}
          {localDir && (
            <FileTree localDir={localDir} commitSha={selectedCommit?.sha} />
          )}

          {/* No local repo warning in sidebar */}
          {!localDir && (
            <div className="p-3">
              <p className="text-xs text-muted-foreground">
                Set local repository path to view commits and files.
              </p>
            </div>
          )}
        </Collapsible.Content>

        <Collapsible.Trigger asChild>
          <button className="flex items-center justify-center w-6 border-r hover:bg-muted transition-colors">
            {isSidebarOpen ? (
              <ChevronLeft className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
          </button>
        </Collapsible.Trigger>
      </Collapsible.Root>

      {/* Right: Main panel */}
      <ScrollFocus
        className="flex-1 overflow-y-auto pl-4"
        panelKey={DIFF_VIEW_PANEL_KEY}
      >
        <div className="space-y-4 p-4">
          {/* PR Header */}
          <div className="flex justify-between items-start">
            <div>
              <h1 className="text-2xl font-semibold">
                {data ? data.title : `Pull Request #${number}`}
              </h1>
              {data && (
                <p className="text-muted-foreground">
                  {data.base.ref} &larr; {data.head.ref}
                </p>
              )}
            </div>
            <div className="flex gap-2">
              {isAuthenticated && data && data.mergeable && (
                <Button
                  onClick={handleMerge}
                  disabled={mergeMutation.isPending}
                  variant="default"
                >
                  {mergeMutation.isPending ? "Merging..." : "Merge PR (Squash)"}
                </Button>
              )}
              <Button onClick={() => refetch()}>Reload</Button>
            </div>
          </div>

          {/* Local repo not set warning */}
          {!localDir && (
            <Alert>
              <AlertTitle>Local Repository Not Set</AlertTitle>
              <AlertDescription>
                Please set the local repository path on the repository page to
                view diffs and commits.
              </AlertDescription>
            </Alert>
          )}

          {/* PR Description (collapsible) */}
          {data && (
            <Collapsible.Root
              open={isDescriptionOpen}
              onOpenChange={setIsDescriptionOpen}
            >
              <div className="rounded-lg border bg-muted/30">
                <Collapsible.Trigger asChild>
                  <button className="flex items-center justify-between w-full p-4 text-left hover:bg-muted/50 transition-colors rounded-lg">
                    <h3 className="text-sm font-medium text-muted-foreground">
                      Description
                    </h3>
                    {isDescriptionOpen ? (
                      <ChevronUp className="w-4 h-4 text-muted-foreground" />
                    ) : (
                      <ChevronDown className="w-4 h-4 text-muted-foreground" />
                    )}
                  </button>
                </Collapsible.Trigger>
                <Collapsible.Content>
                  <div className="px-4 pb-4">
                    {data.body ? (
                      <p className="whitespace-pre-wrap text-sm">{data.body}</p>
                    ) : (
                      <p className="text-sm text-muted-foreground italic">
                        No description provided
                      </p>
                    )}
                  </div>
                </Collapsible.Content>
              </div>
            </Collapsible.Root>
          )}

          {/* Diff Section */}
          {selectedCommit && localDir && (
            <CommitDiffSection
              localDir={localDir}
              commitSha={selectedCommit.sha}
            />
          )}

          {/* No commit selected */}
          {!selectedCommit && data && (
            <p className="text-muted-foreground">
              Select a commit to view changes
            </p>
          )}
        </div>
      </ScrollFocus>
    </main>
  )
}
