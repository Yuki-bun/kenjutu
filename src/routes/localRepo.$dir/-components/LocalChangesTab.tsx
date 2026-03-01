import { useHotkey } from "@tanstack/react-hotkeys"
import { useMemo, useState } from "react"
import { usePanelRef } from "react-resizable-panels"

import { JjCommit } from "@/bindings"
import {
  type CommentContext,
  CommitDiffSection,
  FileDiffItem,
  Header,
  useDiffContext,
} from "@/components/Diff"
import { ErrorDisplay } from "@/components/error"
import { FileTree } from "@/components/FileTree"
import { InlineCommentForm } from "@/components/InlineCommentForm"
import { MarkdownContent } from "@/components/MarkdownContent"
import { Pane, PANEL_KEYS, usePaneManager } from "@/components/Pane"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable"

import { useJjLogGraph } from "../-hooks/useJjLogGraph"
import { useLocalCommentMutations } from "../-hooks/useLocalCommentMutations"
import { CommitGraph } from "./CommitGraph"
import { LocalCommentsSidebar } from "./LocalCommentsSidebar"

type LocalChangesTabProps = {
  localDir: string
}

export function LocalChangesTab({ localDir }: LocalChangesTabProps) {
  const { data, error, isLoading } = useJjLogGraph(localDir)
  const [selectedChangeId, setSelectedChangeId] = useState<string | null>(null)
  const leftSidebarRef = usePanelRef()
  const rightSidebarRef = usePanelRef()
  const isLeftCollapsed = () => leftSidebarRef.current?.isCollapsed() ?? false
  const { focusPane } = usePaneManager()
  const expandLeftAndFocus = (panelKey: string) => {
    leftSidebarRef.current?.expand()
    focusPane(panelKey)
  }

  useHotkey("1", () => expandLeftAndFocus(PANEL_KEYS.commitGraph))
  useHotkey("2", () => expandLeftAndFocus(PANEL_KEYS.fileTree))
  useHotkey("3", () => focusPane(PANEL_KEYS.diffVew))
  useHotkey("4", () => {
    if (rightSidebarRef.current?.isCollapsed()) {
      rightSidebarRef.current.expand()
    } else {
      rightSidebarRef.current?.collapse()
    }
  })

  useHotkey("Mod+B", () => {
    if (isLeftCollapsed()) {
      leftSidebarRef.current?.expand()
    } else {
      leftSidebarRef.current?.collapse()
    }
  })

  // Extract commits from graph rows for lookup and empty-state check
  const commits = useMemo(() => {
    if (!data) return []
    return data.rows
      .filter(
        (row): row is Extract<typeof row, { type: "commit" }> =>
          row.type === "commit",
      )
      .map((row) => row.commit)
  }, [data])

  if (isLoading) {
    return <p className="text-muted-foreground p-4">Loading commits...</p>
  }

  if (error) {
    return <ErrorDisplay error={error} />
  }

  if (!data || commits.length === 0) {
    return (
      <Alert className="mt-4">
        <AlertTitle>No Local Changes</AlertTitle>
        <AlertDescription>
          No mutable commits found. All changes have been pushed.
        </AlertDescription>
      </Alert>
    )
  }

  const selectedCommit = commits.find((c) => c.changeId === selectedChangeId)

  return (
    <ResizablePanelGroup className="flex h-full">
      {/* Left: Commit Graph + File Tree - Collapsible */}
      <ResizablePanel defaultSize="20%" collapsible panelRef={leftSidebarRef}>
        <div className="pb-4 border-b">
          <CommitGraph
            localDir={localDir}
            graph={data}
            selectedChangeId={selectedChangeId ?? null}
            onSelectCommit={(commit) => setSelectedChangeId(commit.changeId)}
          />
        </div>
        <div className="pt-4">
          <FileTree localDir={localDir} commitSha={selectedCommit?.commitId} />
        </div>
      </ResizablePanel>
      <ResizableHandle withHandle />
      {/* Center: Commit details and diff */}
      <ResizablePanel>
        <Pane
          className="h-full min-h-0 overflow-y-auto pl-4"
          panelKey={PANEL_KEYS.diffVew}
        >
          {selectedCommit ? (
            <div className="space-y-4 pt-4 pr-3">
              <CommitDetail commit={selectedCommit} />
              <CommitDiffSection
                localDir={localDir}
                commitSha={selectedCommit.commitId}
              >
                <LocalDiffContent localDir={localDir} />
              </CommitDiffSection>
            </div>
          ) : (
            <p className="text-muted-foreground p-4">
              Select a commit to view changes
            </p>
          )}
        </Pane>
      </ResizablePanel>
      {/* Right: Comments Sidebar - Collapsible */}
      <ResizableHandle withHandle />
      <ResizablePanel
        defaultSize="0%"
        minSize={350}
        collapsible
        panelRef={rightSidebarRef}
      >
        {selectedCommit && (
          <LocalCommentsSidebar
            localDir={localDir}
            changeId={selectedCommit.changeId}
            sha={selectedCommit.commitId}
          />
        )}
      </ResizablePanel>
    </ResizablePanelGroup>
  )
}

function LocalDiffContent({ localDir }: { localDir: string }) {
  const { files, changeId, commitSha } = useDiffContext()

  const { addComment } = useLocalCommentMutations(localDir, changeId, commitSha)

  const commentContext: CommentContext = useMemo(
    () => ({
      onCreateComment: async (params) => {
        await addComment.mutateAsync({
          filePath: params.path,
          side: params.side,
          line: params.line,
          startLine: params.startLine,
          body: params.body,
        })
      },
    }),
    [addComment],
  )

  return (
    <div className="space-y-2">
      <Header />
      <div className="space-y-3">
        {files.map((file) => (
          <FileDiffItem
            key={`${changeId}-${file.newPath || file.oldPath}`}
            file={file}
            commentContext={commentContext}
            InlineCommentForm={InlineCommentForm}
          />
        ))}
      </div>
    </div>
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
