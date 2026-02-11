import { useQuery } from "@tanstack/react-query"
import { createFileRoute, Link } from "@tanstack/react-router"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"
import { getLocalRepoDirs } from "@/lib/repos"

import { useRepositories } from "./-useRepositories"

export const Route = createFileRoute("/")({
  component: RouteComponent,
})

function RouteComponent() {
  return (
    <div className="flex w-full h-full space-x-3">
      <div className="flex-1">
        <GhReposTable />
      </div>
      <div className="flex-1">
        <LocalRepos />
      </div>
    </div>
  )
}

function LocalRepos() {
  const { data } = useQuery({
    queryKey: queryKeys.localRepos(),
    queryFn: getLocalRepoDirs,
  })

  const localRepoDirs = data ?? []
  return (
    <Card className="p-4">
      <CardTitle>Local Repositories</CardTitle>
      <CardContent className="mt-5">
        <ul>
          {localRepoDirs.map((dir) => (
            <li key={dir}>
              <Link to="/localRepo/$dir" params={{ dir }} className="underline">
                {dir}
              </Link>
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  )
}

function GhReposTable() {
  const { isAuthenticated } = useGithub()
  const { data: repos, error, refetch, isLoading } = useRepositories()

  if (!isAuthenticated) {
    return (
      <Alert className="mb-4">
        <AlertTitle>Not Authenticated</AlertTitle>
        <AlertDescription>
          Please sign in with GitHub to view your repositories.
        </AlertDescription>
      </Alert>
    )
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex justify-between">
          <h3 className="w-fit">Github Repositories</h3>
          <Button className="" onClick={() => refetch()}>
            reload
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading && <p>Loading repositories...</p>}

        {error && (
          <Alert variant="destructive" className="mb-4">
            <AlertTitle>Error</AlertTitle>
            <AlertDescription>
              {error instanceof Error
                ? error.message
                : "Failed to load repositories"}
            </AlertDescription>
          </Alert>
        )}

        {repos && (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>owner</TableHead>
                <TableHead>name</TableHead>
                <TableHead>github url</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {repos.map((repo) => (
                <TableRow key={repo.id}>
                  <TableCell>{repo.owner.login}</TableCell>
                  <TableCell>
                    <Link
                      to="/repos/$owner/$repo"
                      params={{ owner: repo.owner.login, repo: repo.name }}
                      search={{ id: repo.node_id }}
                      className="underline"
                    >
                      {repo.name}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <a
                      href={repo.html_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="underline"
                    >
                      {repo.html_url}
                    </a>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  )
}
