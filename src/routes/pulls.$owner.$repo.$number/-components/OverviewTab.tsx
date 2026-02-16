import { useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { MarkdownContent } from "@/components/MarkdownContent"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader } from "@/components/ui/card"
import { queryKeys } from "@/lib/queryKeys"

import { useMergePullRequest } from "../-hooks/useMergePullRequest"
import { usePullRequestDetails } from "../-hooks/usePullRequestDetails"
import { PRChecks } from "./PRChecks"
import { PRComments } from "./PRComments"
import { PRReviewers } from "./PRReviewers"

type OverviewTabProps = {
  localDir: string | null
  owner: string
  repo: string
  number: number
  isAuthenticated: boolean
}

export function OverviewTab({
  owner,
  repo,
  number,
  isAuthenticated,
}: OverviewTabProps) {
  const { data: pullRequest } = usePullRequestDetails(owner, repo, number)
  const { data: prDetails } = usePullRequestDetails(owner, repo, number)

  const mergeMutation = useMergePullRequest()

  const queryClient = useQueryClient()
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
          queryClient.invalidateQueries({
            queryKey: queryKeys.pr(owner, repo, number),
          })
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
    <div className="max-w-7xl mx-auto p-6">
      <div className="flex gap-6">
        {/* Main content area */}
        <div className="flex-1 space-y-6">
          {/* PR Description */}
          {pullRequest && (
            <Card>
              <CardHeader>Description</CardHeader>
              <CardContent>
                <div>
                  {pullRequest.body ? (
                    <MarkdownContent>{pullRequest.body}</MarkdownContent>
                  ) : (
                    <p className="text-muted-foreground">
                      No description provided
                    </p>
                  )}
                </div>
              </CardContent>
            </Card>
          )}

          <PRComments owner={owner} repo={repo} number={number} />

          {/* CI Checks + Merge Actions */}
          <div className="rounded-lg border bg-card">
            <div className="p-4 space-y-4">
              <PRChecks
                owner={owner}
                repo={repo}
                headSha={prDetails?.head.sha}
              />
              {isAuthenticated && pullRequest && pullRequest.mergeable && (
                <div className="pt-4 border-t flex justify-end">
                  <Button
                    onClick={handleMerge}
                    disabled={mergeMutation.isPending}
                    variant="default"
                  >
                    {mergeMutation.isPending
                      ? "Merging..."
                      : "Merge PR (Rebase)"}
                  </Button>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Right sidebar */}
        <div className="w-80 shrink-0">
          <PRReviewers owner={owner} repo={repo} number={number} />
        </div>
      </div>
    </div>
  )
}
