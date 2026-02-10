import { CheckCircle, Clock, XCircle, XOctagon } from "lucide-react"

type CheckStatus = "success" | "failure" | "pending" | "cancelled"

type MockCheck = {
  name: string
  context: string
  status: CheckStatus
  duration?: string
}

const MOCK_CHECKS: MockCheck[] = [
  {
    name: "Build",
    context: "build",
    status: "success",
    duration: "2m 34s",
  },
  {
    name: "Tests",
    context: "unit-tests",
    status: "success",
    duration: "4m 12s",
  },
  {
    name: "Deploy",
    context: "preview",
    status: "pending",
  },
  {
    name: "Lint",
    context: "eslint",
    status: "failure",
    duration: "1m 05s",
  },
]

export function PRChecks() {
  const overallStatus = getOverallStatus(MOCK_CHECKS)
  const { bgColor } = getOverallStatusStyle(overallStatus)

  return (
    <div>
      <div className="flex items-center gap-2 mb-3">
        <div
          className={`w-3 h-3 rounded-full ${bgColor}`}
          title={`Overall status: ${overallStatus}`}
        />
        <h3 className="text-sm font-medium">Checks ({MOCK_CHECKS.length})</h3>
      </div>
      <div className="space-y-3">
        {MOCK_CHECKS.map((check) => (
          <CheckItem key={`${check.name}-${check.context}`} check={check} />
        ))}
      </div>
    </div>
  )
}

function CheckItem({ check }: { check: MockCheck }) {
  const { icon: Icon, color, label } = getStatusInfo(check.status)

  return (
    <div className="flex items-center gap-3">
      <Icon className={`w-4 h-4 shrink-0 ${color}`} />
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium">
          {check.name} / {check.context}
        </div>
        <div className={`text-xs ${color}`}>
          {label}
          {check.duration && ` (${check.duration})`}
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

function getOverallStatus(checks: MockCheck[]): CheckStatus {
  const hasFailure = checks.some((c) => c.status === "failure")
  const hasPending = checks.some((c) => c.status === "pending")
  const allSuccess = checks.every((c) => c.status === "success")

  if (hasFailure) return "failure"
  if (hasPending) return "pending"
  if (allSuccess) return "success"
  return "cancelled"
}

function getOverallStatusStyle(status: CheckStatus) {
  switch (status) {
    case "success":
      return {
        color: "text-green-600 dark:text-green-400",
        bgColor: "bg-green-600 dark:bg-green-400",
      }
    case "failure":
      return {
        color: "text-red-600 dark:text-red-400",
        bgColor: "bg-red-600 dark:bg-red-400",
      }
    case "pending":
      return {
        color: "text-yellow-600 dark:text-yellow-400",
        bgColor: "bg-yellow-600 dark:bg-yellow-400",
      }
    case "cancelled":
      return {
        color: "text-gray-600 dark:text-gray-400",
        bgColor: "bg-gray-600 dark:bg-gray-400",
      }
  }
}
