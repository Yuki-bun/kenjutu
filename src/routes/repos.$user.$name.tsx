import { createFileRoute, Link } from '@tanstack/react-router'
import { commands } from "./../bindings"
import { useFailableQuery, useRpcMutation } from "./../hooks/useRpcQuery"
import { open } from '@tauri-apps/plugin-dialog';

export const Route = createFileRoute('/repos/$user/$name')({
  component: RouteComponent,
})

function RouteComponent() {
  const { user, name } = Route.useParams()

  const { data: repoData, error: repoError, refetch: refetchRepo } = useFailableQuery({
    queryKey: ["repo", { user, name }],
    queryFn: () => commands.getRepoById(user, name)
  })

  const { mutate } = useRpcMutation({
    mutationFn: (dir: string) => commands.setLocalRepo(user, name, dir),
    onSuccess: () => {
      refetchRepo()
    },
    onError: (err) => {
      window.alert(`faield to set local repository directory: ${err}`)
    }
  })

  const { data: prData, error: prError, refetch } = useFailableQuery({
    queryKey: ["pullRequests", user, name],
    queryFn: () => commands.getPullRequests(user, name)
  })

  const handleSelectLocalRepo = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: `Select local repository for ${user}/${name}`,
    });

    if (selected && typeof selected === 'string') {
      mutate(selected);
    }

  };

  return (
    <main className="container">
      {repoError && <p>Error loading repository: {repoError}</p>}
      {!repoData && !repoError && <p>Loading repository data...</p>}

      {repoData && (
        <>
          <h1>Pull Requests: {user}/{name}</h1>
          <p>{repoData.description}</p>
          <p>Local repository: {repoData.local_repo ? repoData.local_repo : "No Set"}
            <button onClick={handleSelectLocalRepo}>Select Local Repository</button>
          </p>

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
