import { useState } from "react"
import { useHotkeys } from "react-hotkeys-hook"

import { JjCommit } from "@/bindings"
import { CommitDiffSection, FileTree } from "@/components/diff"
import { ErrorDisplay } from "@/components/error"
import { MarkdownContent } from "@/components/MarkdownContent"
import { focusPanel, PANEL_KEYS, ScrollFocus } from "@/components/ScrollFocus"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable"

import { useJjLog } from "../-hooks/useJjLog"
import { CommitGraph } from "./CommitGraph"

type LocalChangesTabProps = {
  localDir: string
}

const DIFF_VIEW_PANEL_KEY = "diff-view"

export function LocalChangesTab({ localDir }: LocalChangesTabProps) {
  const { data, error, isLoading } = useJjLog(localDir)
  const [selectedChangeId, setSelectedChangeId] = useState<string | null>(null)
  const [isSidebarOpen, setIsSidebarOpen] = useState(true)

  useHotkeys(
    "1",
    () => {
      if (isSidebarOpen) {
        focusPanel(PANEL_KEYS.commitGraph)
      } else {
        setIsSidebarOpen(true)
        setTimeout(() => focusPanel(PANEL_KEYS.commitGraph), 10)
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

  useHotkeys("3", () => focusPanel(DIFF_VIEW_PANEL_KEY))

  useHotkeys("meta+b", () => setIsSidebarOpen((open) => !open))

  if (isLoading) {
    return <p className="text-muted-foreground p-4">Loading commits...</p>
  }

  if (error) {
    return <ErrorDisplay error={error} />
  }

  if (!data || data.commits.length === 0) {
    return (
      <Alert className="mt-4">
        <AlertTitle>No Local Changes</AlertTitle>
        <AlertDescription>
          No mutable commits found. All changes have been pushed.
        </AlertDescription>
      </Alert>
    )
  }

  const selectedCommit = data.commits.find(
    (c) => c.changeId === selectedChangeId,
  )

  return (
    <ResizablePanelGroup className="flex h-full">
      {/* Left: Commit Graph - Collapsible */}
      <ResizablePanel defaultSize="20%" hidden={!isSidebarOpen}>
        <div className="pb-4 border-b">
          <CommitGraph
            localDir={localDir}
            commits={data.commits}
            selectedChangeId={selectedChangeId ?? null}
            onSelectCommit={(commit) => setSelectedChangeId(commit.changeId)}
          />
        </div>
        <div className="pt-4">
          <FileTree localDir={localDir} commitSha={selectedCommit?.commitId} />
        </div>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel>
        {/* Right: Commit details and diff */}
        <ScrollFocus
          className="h-full min-h-0 overflow-y-auto pl-4"
          panelKey={DIFF_VIEW_PANEL_KEY}
        >
          {selectedCommit ? (
            <div className="space-y-4 pt-4 pr-3">
              <CommitDetail commit={selectedCommit} />
              <CommitDiffSection
                localDir={localDir}
                commitSha={selectedCommit.commitId}
              />
            </div>
          ) : (
            <p className="text-muted-foreground p-4">
              Select a commit to view changes
            </p>
          )}
        </ScrollFocus>
      </ResizablePanel>
    </ResizablePanelGroup>
  )
}

function CommitDetail({ commit }: { commit: JjCommit }) {
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
          <span className="font-medium">Change ID:</span>{" "}
          <code className="bg-muted px-1 rounded">{commit.changeId}</code>
        </p>
        <p>
          <span className="font-medium">Commit:</span>{" "}
          <code className="bg-muted px-1 rounded">
            {commit.commitId.slice(0, 12)}
          </code>
        </p>
        <p>
          <span className="font-medium">Author:</span> {commit.author} &lt;
          {commit.email}&gt;
        </p>
        <p>
          <span className="font-medium">Date:</span> {commit.timestamp}
        </p>
        {commit.isWorkingCopy && (
          <p className="text-green-600 dark:text-green-400 font-medium">
            Working copy
          </p>
        )}
        {commit.isImmutable && (
          <p className="text-amber-600 dark:text-amber-400 font-medium">
            Immutable
          </p>
        )}
      </div>
    </div>
  )
}
