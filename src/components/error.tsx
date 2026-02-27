import { Error as CommandError } from "@/bindings"

import { Alert, AlertDescription, AlertTitle } from "./ui/alert"

type ErrorDisplayProps = {
  error: CommandError
}

export function ErrorDisplay({ error }: ErrorDisplayProps) {
  return (
    <Alert variant="destructive">
      <AlertTitle>Error</AlertTitle>
      <AlertDescription>{getErrorMessage(error)}</AlertDescription>
    </Alert>
  )
}

export function getErrorMessage(error: CommandError): string {
  switch (error.type) {
    case "BadInput":
      return error.message
    case "Repository":
      return `Repository error: ${error.message}`
    case "Git":
      return `Git error: ${error.message}`
    case "Jj":
      return `Jj error: ${error.message}`
    case "FileNotFound":
      return `File not found: ${error.path}`
    case "Internal":
      return "An unexpected error occurred"
    case "MarkerCommit":
      return `MarkerCommit error: ${error.message}`
    case "MergeConflict":
      return `Conflicted commit is not supported: ${error.message}`
    case "CommentCommit":
      return `CommentCommit error: ${error.message}`
    case "SshAuth":
      return `SSH authentication failed: ${error.message}`
  }
}
