import { toast } from "sonner"

import { MarkdownContent } from "@/components/MarkdownContent"
import { Button } from "@/components/ui/button"

import { useMergePullRequest } from "../-hooks/useMergePullRequest"
import { usePullRequest } from "../-hooks/usePullRequest"
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
  localDir,
  isAuthenticated,
}: OverviewTabProps) {
  const { data: pullRequest, refetch } = usePullRequest(
    localDir,
    owner,
    repo,
    number,
  )
  const { data: prDetails } = usePullRequestDetails(owner, repo, number)

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
    <div className="flex-1 overflow-y-auto">
      <div className="max-w-7xl mx-auto p-6">
        <div className="flex gap-6">
          {/* Main content area */}
          <div className="flex-1 space-y-6">
            {/* PR Description */}
            {pullRequest && (
              <div className="rounded-lg border bg-muted/30">
                <button className="flex items-center justify-between w-full p-4 text-left hover:bg-muted/50 transition-colors rounded-lg">
                  <h3 className="text-sm font-medium text-muted-foreground">
                    Description
                  </h3>
                </button>
                <div className="px-4 pb-4">
                  {pullRequest.body ? (
                    <MarkdownContent>{pullRequest.body}</MarkdownContent>
                  ) : (
                    <p className="text-sm text-muted-foreground italic">
                      No description provided
                    </p>
                  )}
                </div>
              </div>
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
                        : "Merge PR (Squash)"}
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
    </div>
  )
}
