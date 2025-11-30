import { useState } from "react";
import { open } from '@tauri-apps/plugin-dialog';
import { setRepository } from "./generated";
import { useMutation } from "@tanstack/react-query";

function App() {
  const [repo, setRepo] = useState("")

  const mutation = useMutation({
    mutationFn: (dir: string) => setRepository({ dir })
  })

  const handleSelectRepository = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Repository"
    });

    if (selected && typeof selected === 'string') {
      setRepo(selected)
      mutation.mutate(selected)
    }
  }

  return (
    <main className="container">
      <h1>Welcome to PR Manager</h1>

      <div style={{ marginTop: '20px' }}>
        <button onClick={handleSelectRepository} disabled={mutation.isPending}>
          {mutation.isPending ? 'Setting repository...' : 'Choose Repository'}
        </button>
      </div>

      {repo && (
        <div style={{ marginTop: '20px' }}>
          <strong>Selected Repository:</strong>
          <p style={{ wordBreak: 'break-all' }}>{repo}</p>
        </div>
      )}

      {mutation.error && (
        <p style={{ color: 'red', marginTop: '20px' }}>
          Error: {mutation.error.message}
        </p>
      )}

      {mutation.isSuccess && (
        <p style={{ color: 'green', marginTop: '20px' }}>
          Repository set successfully!
        </p>
      )}

    </main >
  );
}

export default App;
