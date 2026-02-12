import { createFileRoute } from "@tanstack/react-router"

import { useTab } from "@/hooks/useTab"

import { LocalChangesTab } from "./-components/LocalChangesTab"

export const Route = createFileRoute("/localRepo/$dir")({
  component: RouteComponent,
})

function RouteComponent() {
  const { dir } = Route.useParams()

  const folderName = dir.split(/[/\\]/).pop() || dir
  useTab(`Local: ${folderName}`)

  return <LocalChangesTab localDir={dir} />
}
