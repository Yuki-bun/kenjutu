import { useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { PRCommit } from "@/bindings"
import { CommitDiffSection } from "@/components/Diff"
import { FileTree } from "@/components/FileTree"
import { focusPanel, PANEL_KEYS, ScrollFocus } from "@/components/ScrollFocus"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable"

import { useCommitsInRange } from "../-hooks/useCommitsInRange"
import { usePullRequestDetails } from "../-hooks/usePullRequestDetails"
import { PRCommitList } from "./PRCommitList"
import { PRDiffContent } from "./PRDiffContent"

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
  const prQuery = usePullRequestDetails(owner, repo, prNumber)
  const { data: commits } = useCommitsInRange(
    localDir,
    prQuery.data?.base.sha,
    prQuery.data?.head.sha,
  )
  const [commitSelection, setCommitSelection] =
    useState<CommitSelection | null>(null)
  const [isSidebarOpen, setIsSidebarOpen] = useState(true)

  const selectedCommit = commits?.find((commit: PRCommit) => {
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
      <ResizablePanelGroup orientation="horizontal">
        <ResizablePanel hidden={!isSidebarOpen} defaultSize="20%">
          {/* Commit list */}
          {prQuery.data && localDir && (
            <div className="border-b">
              <PRCommitList
                localDir={localDir}
                commits={commits ?? []}
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
        </ResizablePanel>
        <ResizableHandle withHandle />
        <ResizablePanel defaultSize="80%">
          {/* Right: Diff with inline comments */}
          <ScrollFocus
            className="h-full overflow-y-auto pl-4 min-h-0"
            panelKey={PANEL_KEYS.diffVew}
          >
            <div className="relative space-y-4 p-4 min-h-full">
              {/* Local repo not set warning */}
              {!localDir && (
                <Alert>
                  <AlertTitle>Local Repository Not Set</AlertTitle>
                  <AlertDescription>
                    Please set the local repository path on the repository page
                    to view diffs and commits.
                  </AlertDescription>
                </Alert>
              )}

              {/* Commit Detail + Diff Section */}
              {selectedCommit && localDir && (
                <CommitDiffSection
                  localDir={localDir}
                  commitSha={selectedCommit.sha}
                >
                  <PRDiffContent
                    owner={owner}
                    repo={repo}
                    prNumber={prNumber}
                    currentCommit={selectedCommit}
                    localDir={localDir}
                  />
                </CommitDiffSection>
              )}

              {/* No commit selected */}
              {!selectedCommit && prQuery.data && (
                <p className="text-muted-foreground">
                  Select a commit to view changes
                </p>
              )}
            </div>
          </ScrollFocus>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  )
}
