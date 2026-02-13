import * as Collapsible from "@radix-ui/react-collapsible"
import { ChevronLeft, ChevronRight } from "lucide-react"
import { useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { CommitDiffSection, FileTree } from "@/components/diff"
import { MarkdownContent } from "@/components/MarkdownContent"
import { focusPanel, PANEL_KEYS, ScrollFocus } from "@/components/ScrollFocus"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"

import { PRCommit, usePullRequest } from "../-hooks/usePullRequest"
import { PRCommitList } from "./PRCommitList"

type CommitSelection =
  | {
      type: "change-id"
      changeId: string
    }
  | {
      type: "commit-id"
      commitId: string
    }

type FilesTabProps = {
  localDir: string | null
  owner: string
  repo: string
  prNumber: number
}

export function FilesTab({ localDir, owner, repo, prNumber }: FilesTabProps) {
  const prQuery = usePullRequest(localDir, owner, repo, prNumber)
  const [commitSelection, setCommitSelection] =
    useState<CommitSelection | null>(null)
  const [isSidebarOpen, setIsSidebarOpen] = useState(true)

  const selectedCommit = prQuery.data?.commits.find((commit: PRCommit) => {
    switch (commitSelection?.type) {
      case "change-id":
        return commit.changeId === commitSelection.changeId
      case "commit-id":
        return commit.sha === commitSelection.commitId
      case undefined:
        return false
    }
  })

  // Keyboard shortcuts
  useHotkeys(
    "1",
    () => {
      if (isSidebarOpen) {
        focusPanel(PANEL_KEYS.prCommitList)
      } else {
        setIsSidebarOpen(true)
        setTimeout(() => focusPanel(PANEL_KEYS.prCommitList), 10)
      }
    },
    [isSidebarOpen],
  )

  useHotkeys(
    "2",
    () => {
      if (isSidebarOpen) {
        focusPanel(PANEL_KEYS.fileTree)
      } else {
        setIsSidebarOpen(true)
        setTimeout(() => focusPanel(PANEL_KEYS.fileTree), 10)
      }
    },
    [isSidebarOpen],
  )

  useHotkeys("3", () => focusPanel(PANEL_KEYS.diffVew))

  useHotkeys("meta+b", () => setIsSidebarOpen((open) => !open))

  return (
    <div className="flex h-full">
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
          {/* Commit list */}
          {prQuery.data && localDir && (
            <div className="border-b">
              <PRCommitList
                localDir={localDir}
                commits={prQuery.data.commits}
                selectedCommitSha={selectedCommit?.sha ?? null}
                onSelectCommit={(commit: PRCommit) =>
                  setCommitSelection(
                    commit.changeId
                      ? { type: "change-id", changeId: commit.changeId }
                      : { type: "commit-id", commitId: commit.sha },
                  )
                }
              />
            </div>
          )}

          {/* File tree - only show when commit is selected */}
          {localDir && selectedCommit && (
            <FileTree localDir={localDir} commitSha={selectedCommit.sha} />
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
        panelKey={PANEL_KEYS.diffVew}
      >
        <div className="space-y-4 p-4">
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

          {/* Commit Detail + Diff Section */}
          {selectedCommit && localDir && (
            <>
              <CommitDetail commit={selectedCommit} />
              <CommitDiffSection
                localDir={localDir}
                commitSha={selectedCommit.sha}
              />
            </>
          )}

          {/* No commit selected */}
          {!selectedCommit && prQuery.data && (
            <p className="text-muted-foreground">
              Select a commit to view changes
            </p>
          )}
        </div>
      </ScrollFocus>
    </div>
  )
}

function CommitDetail({ commit }: { commit: PRCommit }) {
  return (
    <div className="p-4 border rounded">
      <h3 className="font-semibold mb-2">
        {commit.summary || "(no description)"}
      </h3>
      {commit.description && (
        <MarkdownContent>{commit.description}</MarkdownContent>
      )}
      <div className="text-sm text-muted-foreground space-y-1 mt-1">
        <p>
          <span className="font-medium">Commit:</span>{" "}
          <code className="bg-muted px-1 rounded">
            {commit.sha.slice(0, 12)}
          </code>
        </p>
        {commit.changeId && (
          <p>
            <span className="font-medium">Change ID:</span>{" "}
            <code className="bg-muted px-1 rounded">{commit.changeId}</code>
          </p>
        )}
      </div>
    </div>
  )
}
