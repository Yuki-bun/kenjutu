import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

type CheckStatus = "success" | "failure" | "pending" | "cancelled"

export type Check = {
  name: string
  context: string
  status: CheckStatus
  duration?: string
  detailsUrl?: string
  appIconUrl?: string
  appName?: string
}

type GitHubCheckRun =
  RestEndpointMethodTypes["checks"]["listForRef"]["response"]["data"]["check_runs"][number]

function mapGitHubCheckToStatus(check: GitHubCheckRun): CheckStatus {
  if (check.status === "queued" || check.status === "in_progress") {
    return "pending"
  }

  switch (check.conclusion) {
    case "success":
    case "neutral":
      return "success"
    case "failure":
    case "timed_out":
    case "action_required":
      return "failure"
    case "cancelled":
    case "skipped":
      return "cancelled"
    default:
      return "pending"
  }
}

function calculateDuration(
  startedAt?: string | null,
  completedAt?: string | null,
): string | undefined {
  if (!startedAt || !completedAt) return undefined

  const start = new Date(startedAt).getTime()
  const end = new Date(completedAt).getTime()
  const durationMs = end - start

  const seconds = Math.floor(durationMs / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)

  if (hours > 0) {
    const remainingMinutes = minutes % 60
    return `${hours}h ${remainingMinutes}m`
  } else if (minutes > 0) {
    const remainingSeconds = seconds % 60
    return `${minutes}m ${remainingSeconds}s`
  } else {
    return `${seconds}s`
  }
}

function isCICDCheck(check: GitHubCheckRun): boolean {
  const checkName = check.name.toLowerCase()

  const cicdPatterns = [
    "test",
    "lint",
    "build",
    "typecheck",
    "type-check",
    "format",
    "cargo",
    "npm",
    "pnpm",
    "yarn",
    "compile",
    "validate",
    "check",
    "ci",
    "verify",
  ]

  const excludedPatterns = [
    "codeql",
    "security",
    "dependency",
    "dependabot",
    "cleanup",
    "upload",
    "download",
    "cache",
    "artifact",
    "setup",
    "prepare",
    "agent",
    "autovalidate",
  ]

  if (excludedPatterns.some((pattern) => checkName.includes(pattern))) {
    return false
  }

  return cicdPatterns.some((pattern) => checkName.includes(pattern))
}

export function usePullRequestChecks(
  owner: string,
  repo: string,
  headSha: string | undefined,
) {
  const { isAuthenticated, octokit } = useGithub()

  return useQuery({
    queryKey: queryKeys.pullRequestChecks(owner, repo, headSha || ""),
    queryFn: async (): Promise<Check[]> => {
      if (!headSha) return []

      const { data } = await octokit!.checks.listForRef({
        owner,
        repo,
        ref: headSha,
      })

      const cicdChecks = data.check_runs.filter(isCICDCheck)

      return cicdChecks.map((check) => ({
        name: check.name,
        context: check.name,
        status: mapGitHubCheckToStatus(check),
        duration: calculateDuration(check.started_at, check.completed_at),
        detailsUrl: check.html_url ?? undefined,
        appIconUrl: check.app?.owner.avatar_url ?? undefined,
        appName: check.app?.name ?? undefined,
      }))
    },
    enabled: !!octokit && isAuthenticated && !!headSha,
    refetchInterval: 30 * 1000,
  })
}
