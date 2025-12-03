import { createFileRoute, Link } from '@tanstack/react-router'
import { commands } from "./../bindings"
import { useFailableQuery } from "./../hooks/useRpcQuery"

export const Route = createFileRoute('/repos/$user/$name')({
  component: RouteComponent,
})

function RouteComponent() {
  const { user, name } = Route.useParams()

  const { data, error, refetch } = useFailableQuery({
    queryKey: ["pullRequests", user, name],
    queryFn: () => commands.getPullRequests(user, name)
  })

  return (
    <main className="container">
      <h1>Pull Requests: {user}/{name}</h1>

      <button onClick={() => refetch()}>reload</button>

      {!data && !error && <p>Loading pull requests...</p>}

      {data && data.length > 0 && (
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>Title</th>
              <th>Author</th>
              <th>GitHub URL</th>
            </tr>
          </thead>
          <tbody>
            {data.map((pr) => (
              <tr key={pr.id}>
                <td>{pr.id}</td>
                <td>
                  <Link
                    to="/repos/$user/$name/prs/$id"
                    params={{ user, name, id: String(pr.id) }}
                  >
                    {pr.title ?? `PR #${pr.id}`}
                  </Link>
                </td>
                <td>
                  {pr.author ? (
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <img
                        src={pr.author.avatar_url}
                        alt={pr.author.login}
                        style={{ width: '24px', height: '24px', borderRadius: '50%' }}
                      />
                      <span>{pr.author.name ?? pr.author.login}</span>
                    </div>
                  ) : (
                    <span style={{ fontStyle: 'italic', color: '#666' }}>Unknown</span>
                  )}
                </td>
                <td>
                  <a href={pr.github_url ?? undefined} target="_blank" rel="noopener noreferrer">
                    {pr.github_url}
                  </a>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {data && data.length === 0 && (
        <p>No pull requests found for this repository.</p>
      )}

      {error && <p>{error}</p>}
    </main>
  )
}
