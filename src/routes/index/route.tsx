import { useQuery } from "@tanstack/react-query"
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router"
import { useMemo, useRef, useState } from "react"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
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
  const navigate = useNavigate()
  const [filter, setFilter] = useState("")
  const inputRef = useRef<HTMLInputElement>(null)
  const cardRef = useRef<HTMLDivElement>(null)

  const { data: allRepos, error, refetch, isLoading } = useRepositories()

  const filteredRepos = useMemo(() => {
    if (!allRepos) return []
    if (!filter) return allRepos

    const lowerFilter = filter.toLowerCase()
    return allRepos.filter(
      (repo) =>
        repo.name.toLowerCase().includes(lowerFilter) ||
        repo.owner.login.toLowerCase().includes(lowerFilter) ||
        repo.full_name.toLowerCase().includes(lowerFilter),
    )
  }, [allRepos, filter])

  const handleCardKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "/" && document.activeElement !== inputRef.current) {
      e.preventDefault()
      inputRef.current?.focus()
    }
  }

  const handleInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Escape") {
      inputRef.current?.blur()
    }
  }

  const handleRowKeyDown = (
    e: React.KeyboardEvent<HTMLTableRowElement>,
    owner: string,
    name: string,
    nodeId: string,
  ) => {
    if (e.key === "Enter") {
      navigate({
        to: "/repos/$owner/$repo",
        params: { owner, repo: name },
        search: { id: nodeId },
      })
    }
  }

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
    <Card ref={cardRef} onKeyDown={handleCardKeyDown}>
      <CardHeader>
        <div className="flex justify-between">
          <h3 className="w-fit">Github Repositories</h3>
          <Button className="" onClick={() => refetch()}>
            reload
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <Input
          ref={inputRef}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          onKeyDown={handleInputKeyDown}
          placeholder="Filter repositories..."
          className="mb-4"
        />

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

        {filteredRepos && (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>owner</TableHead>
                <TableHead>name</TableHead>
                <TableHead>github url</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredRepos.map((repo) => (
                <TableRow
                  key={repo.id}
                  tabIndex={0}
                  onKeyDown={(e) =>
                    handleRowKeyDown(
                      e,
                      repo.owner.login,
                      repo.name,
                      repo.node_id,
                    )
                  }
                  className="focus:outline-none focus:bg-muted/50 cursor-pointer"
                >
                  <TableCell>{repo.owner.login}</TableCell>
                  <TableCell>
                    <Link
                      to="/repos/$owner/$repo"
                      params={{ owner: repo.owner.login, repo: repo.name }}
                      search={{ id: repo.node_id }}
                      className="underline"
                      tabIndex={-1}
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
                      tabIndex={-1}
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
