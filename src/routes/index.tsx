import { createFileRoute, Link } from '@tanstack/react-router'
import { commands } from "./../bindings"
import { useFailableQuery } from "./../hooks/useRpcQuery";
import { Route as RepoRoute } from './repos.$user.$name'

export const Route = createFileRoute('/')({
  component: RouteComponent,
})

function RouteComponent() {

  const { data, error, refetch } = useFailableQuery({
    queryKey: ["repository"],
    queryFn: () => commands.getReposiotires()
  })


  return (
    <main className="container">
      <h1>Welcome to PR Manager</h1>
      <button
        onClick={() => refetch()}
      >
        reload
      </button>

      <table>
        <thead>
          <tr>
            <th>owner</th>
            <th>name</th>
            <th>github url</th>
          </tr>
        </thead>
        <tbody>
          {data && data.map((repo) =>
            <tr>
              <th>{repo.owner_name}</th>
              <td><Link to={RepoRoute.to}
                params={{
                  name: repo.name,
                  user: repo.owner_name
                }}
              >{repo.name}</Link></td>
              <td><a href={repo.html_url} >{repo.html_url}</a></td>
            </tr>
          )}
        </tbody>
      </table>
      {error && (
        <p>{error}</p>
      )}
    </main >
  );
}
