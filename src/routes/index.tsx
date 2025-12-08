import { createFileRoute, Link } from '@tanstack/react-router'
import { commands } from "./../bindings"
import { useFailableQuery } from "./../hooks/useRpcQuery";
import { Route as RepoRoute } from './repos.$user.$name'
import { Button } from "@/components/ui/button"
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

export const Route = createFileRoute('/')({
  component: RouteComponent,
})

function RouteComponent() {

  const { data, error, refetch } = useFailableQuery({
    queryKey: ["repository"],
    queryFn: () => commands.getReposiotires()
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
                <TableRow key={repo.name}>
                  <TableCell>{repo.owner_name}</TableCell>
                  <TableCell>
                    <Link to={RepoRoute.to}
                      params={{
                        name: repo.name,
                        user: repo.owner_name
                      }}
                    >{repo.name}</Link>
                  </TableCell>
                  <TableCell>
                    <a href={repo.html_url} target="_blank" rel="noopener noreferrer" className="underline">
                      {repo.html_url}
                    </a>
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
          {error && (
            <p className="text-red-500 mt-4">{error}</p>
          )}
        </CardContent>
        <CardFooter>
          {/* Optional footer content */}
        </CardFooter>
      </Card>
    </main >
  );
}
