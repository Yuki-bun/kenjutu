import { commands } from "./bindings"
import { useFailableQuery } from "./hooks/useRpcQuery";



function App() {

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
            <th>name</th>
            <th>url</th>
          </tr>
        </thead>
        <tbody>
          {data && data.map((repo) =>
            <tr>
              <td>{repo.name}</td>
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


export default App;
