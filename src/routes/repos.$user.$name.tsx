import { createFileRoute, Link } from '@tanstack/react-router'
import { commands } from "./../bindings"
import { useFailableQuery } from "./../hooks/useRpcQuery"

export const Route = createFileRoute('/repos/$user/$name')({
  component: RouteComponent,
})

function RouteComponent() {
  const { user, name } = Route.useParams()

  const { data: repoData, error: repoError } = useFailableQuery({
    queryKey: ["repo", { user, name }],
    queryFn: () => commands.getRepoById(user, name)
  })

  const { data: prData, error: prError, refetch } = useFailableQuery({
    queryKey: ["pullRequests", user, name],
    queryFn: () => commands.getPullRequests(user, name)
  })

  return (
    <main className="container">
      {repoError && <p>Error loading repository: {repoError.Internal}</p>}
      {!repoData && !repoError && <p>Loading repository data...</p>}

      {repoData && (
        <>
          <h1>Pull Requests: {user}/{name}</h1>
          <p>{repoData.description}</p>
          <p>Local repository: {repoData.local_repo_set ? "Yes" : "No"}</p>

          <button onClick={() => refetch()}>reload</button>

          {!prData && !prError && <p>Loading pull requests...</p>}

          {prData && prData.length > 0 && (
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
                {prData.map((pr) => (
                  <tr key={pr.id}>
                    <td>{pr.id}</td>
                    <td>
                      <Link
                        to="/repos/$user/$name"
                        params={{ user, name }}
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
        </>
      )}


      {prData && prData.length === 0 && (
        <p>No pull requests found for this repository.</p>
      )}

      {prError && <p>{prError}</p>}
    </main>
  )
}
