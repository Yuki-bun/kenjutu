import { useHotkey } from "@tanstack/react-hotkeys"
import { useState } from "react"
import { usePanelRef } from "react-resizable-panels"

import { PRCommit } from "@/bindings"
import { CommitDiffSection } from "@/components/Diff"
import { FileTree } from "@/components/FileTree"
import { MarkdownContent } from "@/components/MarkdownContent"
import { Pane, PANEL_KEYS, usePaneManager } from "@/components/Pane"
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
  const repository = prQuery.data?.base.repo
  const remoteUrls = repository
    ? [repository.ssh_url, repository.clone_url]
    : []

  const { data: commits } = useCommitsInRange(
    localDir,
    prQuery.data?.base.sha,
    prQuery.data?.head.sha,
    remoteUrls,
  )
  const [commitSelection, setCommitSelection] =
    useState<CommitSelection | null>(null)
  const leftSidebarRef = usePanelRef()
  const rightSidebarRef = usePanelRef()
  const { focusPane } = usePaneManager()

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

  useHotkey("1", () => {
    if (!leftSidebarRef.current?.isCollapsed()) {
      focusPane(PANEL_KEYS.prCommitList)
    } else {
      leftSidebarRef.current.expand()
      setTimeout(() => focusPane(PANEL_KEYS.prCommitList), 10)
    }
  })
  useHotkey("2", () => {
    if (!leftSidebarRef.current?.isCollapsed()) {
      focusPane(PANEL_KEYS.fileTree)
    } else {
      leftSidebarRef.current.expand()
      setTimeout(() => focusPane(PANEL_KEYS.fileTree), 10)
    }
  })
  useHotkey("3", () => focusPane(PANEL_KEYS.diffVew))
  useHotkey("4", () => {
    if (rightSidebarRef.current?.isCollapsed()) {
      rightSidebarRef.current.expand()
    } else {
      rightSidebarRef.current?.collapse()
    }
  })
  useHotkey("Mod+B", () => {
    if (leftSidebarRef.current?.isCollapsed()) {
      leftSidebarRef.current.expand()
    } else {
      leftSidebarRef.current?.collapse()
    }
  })

  return (
    <div className="flex h-full">
      {/* Left: Collapsible Sidebar */}
      <ResizablePanelGroup orientation="horizontal">
        <ResizablePanel panelRef={leftSidebarRef} defaultSize="20%" collapsible>
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
          <Pane
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
                      currentCommit={selectedCommit}
                      localDir={localDir}
                      remoteUrls={remoteUrls}
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
          </Pane>
        </ResizablePanel>
        <ResizableHandle withHandle />
        <ResizablePanel
          panelRef={rightSidebarRef}
          defaultSize="20%"
          collapsible
        >
          {/* Right: Review Comments Sidebar */}
          {selectedCommit && (
            <ReviewCommentsSidebar
              currentCommit={selectedCommit}
              localDir={localDir}
              owner={owner}
              repo={repo}
              prNumber={prNumber}
              remoteUrls={remoteUrls}
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
