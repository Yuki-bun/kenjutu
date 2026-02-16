import { useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { PRCommit } from "@/bindings"
import { CommitDiffSection } from "@/components/Diff"
import { FileTree } from "@/components/FileTree"
import { MarkdownContent } from "@/components/MarkdownContent"
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
import { ReviewCommentsSidebar } from "./ReviewCommentsSidebar"

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
  const [isCommentsOpen, setIsCommentsOpen] = useState(true)

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

  useHotkeys("meta+e", () => setIsCommentsOpen((open) => !open))

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
        <ResizablePanel defaultSize="60%">
          {/* Center: Main panel */}
          <ScrollFocus
            className="h-full overflow-y-auto pl-4"
            panelKey={PANEL_KEYS.diffVew}
          >
            <div className="space-y-4 p-4">
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
                <>
                  <CommitDetail commit={selectedCommit} />
                  <CommitDiffSection
                    localDir={localDir}
                    commitSha={selectedCommit.sha}
                  >
                    <PRDiffContent
                      owner={owner}
                      repo={repo}
                      prNumber={prNumber}
                    />
                  </CommitDiffSection>
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
        </ResizablePanel>
        <ResizableHandle withHandle />
        <ResizablePanel hidden={!isCommentsOpen} defaultSize="20%">
          {/* Right: Review Comments Sidebar */}
          {selectedCommit && (
            <ReviewCommentsSidebar
              currentCommit={selectedCommit}
              localDir={localDir}
              owner={owner}
              repo={repo}
              prNumber={prNumber}
            />
          )}
        </ResizablePanel>
      </ResizablePanelGroup>
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
        <p>
          <span className="font-medium">Change ID:</span>{" "}
          <code className="bg-muted px-1 rounded">{commit.changeId}</code>
        </p>
      </div>
    </div>
  )
}
