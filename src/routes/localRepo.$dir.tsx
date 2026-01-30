import { createFileRoute } from "@tanstack/react-router"

import { LocalChangesTab } from "@/components/LocalChangesTab"

export const Route = createFileRoute("/localRepo/$dir")({
  component: RouteComponent,
})

function RouteComponent() {
  const { dir } = Route.useParams()
  return <LocalChangesTab localDir={dir} />
}
