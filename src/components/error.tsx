import { CommandError } from "@/bindings"
import { Alert, AlertDescription, AlertTitle } from "./ui/alert"

type ErrorDisplayProps = {
  error: CommandError
}

export function ErrorDisplay({ error }: ErrorDisplayProps) {

  return (
    <Alert variant="destructive" >
      <AlertTitle>Error</AlertTitle>
      <AlertDescription>
        {error.type === 'Internal' ? (
          <p>unkwon errror has occured</p>
        ) : (
          <p>{error.description}</p>
        )}
      </AlertDescription>
    </Alert>
  )
}

