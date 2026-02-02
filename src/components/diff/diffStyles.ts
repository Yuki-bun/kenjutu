import { DiffLineType, FileChangeStatus } from "@/bindings"

export function getStatusStyle(status: FileChangeStatus): {
  bgColor: string
  textColor: string
  label: string
} {
  switch (status) {
    case "added":
      return {
        bgColor: "bg-green-100 dark:bg-green-900",
        textColor: "text-green-800 dark:text-green-200",
        label: "Added",
      }
    case "modified":
      return {
        bgColor: "bg-blue-100 dark:bg-blue-900",
        textColor: "text-blue-800 dark:text-blue-200",
        label: "Modified",
      }
    case "deleted":
      return {
        bgColor: "bg-red-100 dark:bg-red-900",
        textColor: "text-red-800 dark:text-red-200",
        label: "Deleted",
      }
    case "renamed":
      return {
        bgColor: "bg-purple-100 dark:bg-purple-900",
        textColor: "text-purple-800 dark:text-purple-200",
        label: "Renamed",
      }
    case "copied":
      return {
        bgColor: "bg-yellow-100 dark:bg-yellow-900",
        textColor: "text-yellow-800 dark:text-yellow-200",
        label: "Copied",
      }
    case "typechange":
      return {
        bgColor: "bg-orange-100 dark:bg-orange-900",
        textColor: "text-orange-800 dark:text-orange-200",
        label: "Type",
      }
    default:
      return {
        bgColor: "bg-gray-100 dark:bg-gray-900",
        textColor: "text-gray-800 dark:text-gray-200",
        label: "Changed",
      }
  }
}

export function getLineStyle(lineType: DiffLineType): {
  bgColor: string
} {
  switch (lineType) {
    case "addition":
      return {
        bgColor: "bg-green-50 dark:bg-green-950/30",
      }
    case "deletion":
      return {
        bgColor: "bg-red-50 dark:bg-red-950/30",
      }
    case "context":
    case "addeofnl":
    case "deleofnl":
    default:
      return {
        bgColor: "bg-background",
      }
  }
}
