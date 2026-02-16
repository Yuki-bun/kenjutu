import { MarkdownContent } from "@/components/MarkdownContent"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader } from "@/components/ui/card"

import { useDeleteBranch } from "../-hooks/useDeleteBranch"
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

  const mergeMutation = useMergePullRequest()

  const deleteBranch = useDeleteBranch({
    owner,
    repo,
    branch: pullRequest?.head.ref,
  })

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
                headSha={pullRequest?.head.sha}
              />
              {isAuthenticated && pullRequest && pullRequest.mergeable && (
                <Button
                  onClick={() =>
                    mergeMutation.mutate({ owner, repo, pullNumber: number })
                  }
                  disabled={mergeMutation.isPending}
                  variant="default"
                >
                  {mergeMutation.isPending ? "Merging..." : "Merge PR (Rebase)"}
                </Button>
              )}
              {pullRequest?.merged && deleteBranch && (
                <Button
                  onClick={() => deleteBranch.mutate()}
                  disabled={deleteBranch.isPending}
                  variant="destructive"
                >
                  Delete branch
                </Button>
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
