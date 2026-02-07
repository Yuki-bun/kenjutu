import * as Collapsible from "@radix-ui/react-collapsible"
import { ChevronLeft, ChevronRight } from "lucide-react"
import { useState } from "react"

import { JjCommit } from "@/bindings"
import { CommitDiffSection, FileTree } from "@/components/diff"
import { ErrorDisplay } from "@/components/error"
import { ScrollFocus } from "@/components/ScrollFocus"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { useJjLog } from "@/hooks/useJjLog"

import { CommitGraph } from "./-CommitGraph"

type LocalChangesTabProps = {
  localDir: string
}

export function LocalChangesTab({ localDir }: LocalChangesTabProps) {
  const { data, error, isLoading } = useJjLog(localDir)
  const [selectedChangeId, setSelectedChangeId] = useState<string | null>(null)
  const [isGraphOpen, setIsGraphOpen] = useState(true)

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
    <div className="flex h-full">
      {/* Left: Commit Graph - Collapsible */}
      <Collapsible.Root
        open={isGraphOpen}
        onOpenChange={setIsGraphOpen}
        className="flex shrink-0 h-full"
      >
        <Collapsible.Content className="w-96 border-r pr-4 overflow-y-auto">
          <div className="pb-4 border-b">
            <CommitGraph
              localDir={localDir}
              commits={data.commits}
              selectedChangeId={selectedChangeId ?? null}
              onSelectCommit={(commit) => setSelectedChangeId(commit.changeId)}
            />
          </div>
          <div className="pt-4">
            <FileTree
              localDir={localDir}
              commitSha={selectedCommit?.commitId}
            />
          </div>
        </Collapsible.Content>
        <Collapsible.Trigger asChild>
          <button className="flex items-center justify-center w-6 border-r hover:bg-muted transition-colors">
            {isGraphOpen ? (
              <ChevronLeft className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
          </button>
        </Collapsible.Trigger>
      </Collapsible.Root>

      {/* Right: Commit details and diff */}
      <ScrollFocus className="flex-1 overflow-y-auto pl-4">
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
        <p className="whitespace-pre-wrap text-sm text-muted-foreground mb-3 border-l-2 border-muted pl-2">
          {commit.description}
        </p>
      )}
      <div className="text-sm text-muted-foreground space-y-1">
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
