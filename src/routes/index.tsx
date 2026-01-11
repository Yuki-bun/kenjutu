import { createFileRoute, Link } from '@tanstack/react-router'
import { commands } from "./../bindings"
import { useFailableQuery } from "./../hooks/useRpcQuery"
import { Button } from '@/components/ui/button'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { ErrorDisplay } from '@/components/error';

export const Route = createFileRoute('/')({
  component: RouteComponent,
})

function RouteComponent() {

  const { data, error, refetch } = useFailableQuery({
    queryKey: ["repository"],
    queryFn: () => commands.getRepositories()
  })


  return (
    <main className="min-h-screen w-full p-4">
      <Card className="w-full h-full">
        <CardHeader>
          <CardTitle>Welcome to PR Manager</CardTitle>
          <CardDescription>Manage your GitHub Pull Requests with ease.</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex justify-end mb-4">
            <Button onClick={() => refetch()}>
              reload
            </Button>
          </div>

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>owner</TableHead>
                <TableHead>name</TableHead>
                <TableHead>github url</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data && data.map((repo) =>
                <TableRow key={repo.nodeId}>
                  <TableCell>{repo.ownerName}</TableCell>
                  <TableCell>
                    <Link
                      to="/repos/$nodeId"
                      params={{ nodeId: repo.nodeId }}
                      className="underline"
                    >
                      {repo.name}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <a href={repo.htmlUrl} target="_blank" rel="noopener noreferrer" className="underline">
                      {repo.htmlUrl}
                    </a>
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
          {error && <ErrorDisplay error={error} />}
        </CardContent>
        <CardFooter>
          {/* Optional footer content */}
        </CardFooter>
      </Card>
    </main >
  );
}
