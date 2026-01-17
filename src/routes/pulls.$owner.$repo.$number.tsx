import { useEffect, useState } from "react"
import { useFailableQuery } from "@/hooks/useRpcQuery"
import { useMergePullRequest } from "@/hooks/useMergePullRequest"
import { usePullRequest } from "@/hooks/usePullRequest"
import { useGithub } from "@/context/GithubContext"
import { createFileRoute } from "@tanstack/react-router"
import { toast } from "sonner"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { CommitDiffSection } from "@/components/CommitDiffSection"
import { cn } from "@/lib/utils"

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

type CommitSelection = {
  commitSha: string
  chagneId: string | null
}

function RouteComponent() {
  const { number, owner, repo } = Route.useParams()
  const { repoId } = Route.useSearch()
  const { isAuthenticated } = useGithub()
  const [selectedCommit, setSelectedCommit] = useState<CommitSelection | null>(
    null,
  )
  const [expandedDescriptions, setExpandedDescriptions] = useState<Set<string>>(
    new Set(),
  )

  const toggleDescription = (sha: string) => {
    setExpandedDescriptions((prev) => {
      const next = new Set(prev)
      if (next.has(sha)) {
        next.delete(sha)
      } else {
        next.add(sha)
      }
      return next
    })
  }

  const { data, error, isLoading, refetch } = usePullRequest(
    repoId,
    owner,
    repo,
    Number(number),
  )

  useEffect(() => {
    if (!data) {
      return
    }
    setSelectedCommit((selectedCommit) => {
      if (!selectedCommit) {
        return null
      }
      if (data.commits.find((c) => c.sha === selectedCommit?.commitSha)) {
        return selectedCommit
      }
      const changeId = selectedCommit.chagneId
      if (!changeId) {
        return null
      }
      const newCommit = data.commits.find((c) => c.changeId === changeId)
      if (newCommit) {
        return { commitSha: newCommit.sha, chagneId: newCommit.changeId }
      } else {
        return null
      }
    })
  }, [data])

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

  return (
    <main className="min-h-screen w-full p-4">
      <Card className="w-full h-full">
        <CardHeader>
          <div className="flex justify-between">
            <CardTitle>
              {data ? data.title : `Pull Request #${number}`}
            </CardTitle>
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
          {data && (
            <CardDescription>
              {data.baseBranch} ← {data.headBranch}
            </CardDescription>
          )}
        </CardHeader>
        <CardContent>
          {/* Loading State */}
          {isLoading && (
            <p className="text-muted-foreground">Loading pull request...</p>
          )}

          {/* Error State */}
          {error && (
            <Alert variant="destructive">
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>
                <p>{error instanceof Error ? error.message : String(error)}</p>
              </AlertDescription>
            </Alert>
          )}

          {/* Success State */}
          {data && (
            <div className="space-y-6">
              {/* PR Body Section */}
              <div className="rounded-lg border bg-muted/30 p-4">
                <h3 className="text-sm font-medium text-muted-foreground mb-2">
                  Description
                </h3>
                {data.body ? (
                  <p className="whitespace-pre-wrap text-sm">{data.body}</p>
                ) : (
                  <p className="text-sm text-muted-foreground italic">
                    No description provided
                  </p>
                )}
              </div>

              {/* Commits Section */}
              <div className="space-y-2">
                <h3 className="text-sm font-medium text-muted-foreground">
                  Commits ({data.commits.length})
                </h3>

                {data.commits.length === 0 ? (
                  <Alert>
                    <AlertTitle>No Commits</AlertTitle>
                    <AlertDescription>
                      No commits found in this pull request.
                    </AlertDescription>
                  </Alert>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Message</TableHead>
                        <TableHead className="hidden sm:table-cell w-[100px]">
                          Change ID
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {data.commits.map((commit) => (
                        <TableRow
                          key={commit.sha}
                          onClick={() =>
                            setSelectedCommit({
                              chagneId: commit.changeId,
                              commitSha: commit.sha,
                            })
                          }
                          className={cn(
                            "cursor-pointer hover:bg-muted/50 transition-colors",
                            selectedCommit?.commitSha === commit.sha &&
                              "bg-muted",
                          )}
                        >
                          <TableCell>
                            <div className="flex flex-col gap-1">
                              <div className="flex items-center gap-2">
                                <span>{commit.summary}</span>
                                {commit.description && (
                                  <Button
                                    variant="ghost"
                                    size="sm"
                                    className="h-6 px-2 text-xs text-muted-foreground"
                                    onClick={(e) => {
                                      e.stopPropagation()
                                      toggleDescription(commit.sha)
                                    }}
                                  >
                                    {expandedDescriptions.has(commit.sha)
                                      ? "▼"
                                      : "▶"}
                                  </Button>
                                )}
                              </div>
                              {expandedDescriptions.has(commit.sha) &&
                                commit.description && (
                                  <p className="whitespace-pre-wrap text-sm text-muted-foreground pl-2 border-l-2 border-muted">
                                    {commit.description}
                                  </p>
                                )}
                            </div>
                          </TableCell>
                          <TableCell className="hidden sm:table-cell font-mono text-xs text-muted-foreground">
                            {commit.changeId || "-"}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                )}
              </div>

              {/* Diff Section */}
              {selectedCommit && (
                <CommitDiffSection
                  repoId={repoId}
                  prNumber={Number(number)}
                  commitSha={selectedCommit.commitSha}
                />
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </main>
  )
}
