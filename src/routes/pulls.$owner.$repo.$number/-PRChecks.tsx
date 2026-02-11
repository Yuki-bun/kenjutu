import { CheckCircle, Clock, XCircle, XOctagon } from "lucide-react"

import { type Check, usePullRequestChecks } from "./-usePullRequestChecks"

type CheckStatus = "success" | "failure" | "pending" | "cancelled"

type PRChecksProps = {
  owner: string
  repo: string
  headSha: string | undefined
}

function ChecksProgressIndicator({ checks }: { checks: Check[] }) {
  if (checks.length === 0) return null

  const statusCounts = {
    success: checks.filter((c) => c.status === "success").length,
    failure: checks.filter((c) => c.status === "failure").length,
    pending: checks.filter((c) => c.status === "pending").length,
    cancelled: checks.filter((c) => c.status === "cancelled").length,
  }

  const total = checks.length
  const successPct = (statusCounts.success / total) * 100
  const failurePct = (statusCounts.failure / total) * 100
  const pendingPct = (statusCounts.pending / total) * 100
  const cancelledPct = (statusCounts.cancelled / total) * 100

  const segments = [
    { count: statusCounts.success, pct: successPct, color: "rgb(34, 197, 94)" },
    { count: statusCounts.failure, pct: failurePct, color: "rgb(239, 68, 68)" },
    { count: statusCounts.pending, pct: pendingPct, color: "rgb(234, 179, 8)" },
    {
      count: statusCounts.cancelled,
      pct: cancelledPct,
      color: "rgb(107, 114, 128)",
    },
  ]

  const gradientStops = segments
    .filter((s) => s.count > 0)
    .reduce<Array<{ stop: string; endPct: number }>>((acc, segment) => {
      const startPct = acc.length > 0 ? acc[acc.length - 1].endPct : 0
      const endPct = startPct + segment.pct
      return [
        ...acc,
        {
          stop: `${segment.color} ${startPct}% ${endPct}%`,
          endPct,
        },
      ]
    }, [])
    .map((s) => s.stop)

  return (
    <div
      className="shrink-0 rounded-full"
      style={{
        width: "16px",
        height: "16px",
        background: `conic-gradient(from -90deg, ${gradientStops.join(", ")})`,
        mask: "radial-gradient(circle, transparent 0%, transparent 35%, black 35%)",
        WebkitMask:
          "radial-gradient(circle, transparent 0%, transparent 35%, black 35%)",
      }}
    />
  )
}

export function PRChecks({ owner, repo, headSha }: PRChecksProps) {
  const {
    data: checks,
    isLoading,
    error,
  } = usePullRequestChecks(owner, repo, headSha)

  return (
    <div>
      <div className="flex items-center gap-2 mb-3">
        {checks && checks.length > 0 && (
          <ChecksProgressIndicator checks={checks} />
        )}
        <h3 className="text-base font-semibold">
          Checks {checks && `(${checks.length})`}
        </h3>
      </div>
      <div className="space-y-2.5 max-h-[280px] overflow-y-auto pr-2">
        {isLoading && (
          <div className="text-sm text-muted-foreground">Loading checks...</div>
        )}
        {error && (
          <div className="text-sm text-red-600 dark:text-red-400">
            Failed to load checks
          </div>
        )}
        {checks && checks.length === 0 && (
          <div className="text-sm text-muted-foreground">
            No CI/CD checks configured
          </div>
        )}
        {checks &&
          checks.length > 0 &&
          checks.map((check) => (
            <CheckItem key={`${check.name}-${check.context}`} check={check} />
          ))}
      </div>
    </div>
  )
}

function CheckItem({ check }: { check: Check }) {
  const { icon: Icon, color, label } = getStatusInfo(check.status)

  return (
    <div className="flex items-center gap-2.5 py-1">
      {check.appIconUrl ? (
        <img
          src={check.appIconUrl}
          alt={check.appName || "App icon"}
          className="w-4 h-4 shrink-0 rounded"
        />
      ) : (
        <Icon className={`w-4 h-4 shrink-0 ${color}`} />
      )}
      <div className="flex-1 min-w-0 flex items-baseline gap-2">
        {check.detailsUrl ? (
          <a
            href={check.detailsUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="text-xs font-medium hover:underline"
          >
            {check.name}
          </a>
        ) : (
          <div className="text-xs font-medium">{check.name}</div>
        )}
        <div className={`text-xs ${color}`}>
          {label}
          {check.duration && ` · ${check.duration}`}
        </div>
      </div>
    </div>
  )
}

function getStatusInfo(status: CheckStatus) {
  switch (status) {
    case "success":
      return {
        icon: CheckCircle,
        color: "text-green-600 dark:text-green-400",
        label: "Success",
      }
    case "failure":
      return {
        icon: XCircle,
        color: "text-red-600 dark:text-red-400",
        label: "Failed",
      }
    case "pending":
      return {
        icon: Clock,
        color: "text-yellow-600 dark:text-yellow-400",
        label: "In progress",
      }
    case "cancelled":
      return {
        icon: XOctagon,
        color: "text-gray-600 dark:text-gray-400",
        label: "Cancelled",
      }
  }
}
