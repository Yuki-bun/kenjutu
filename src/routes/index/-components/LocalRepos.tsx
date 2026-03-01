import { useHotkey } from "@tanstack/react-hotkeys"
import { Link, useNavigate } from "@tanstack/react-router"
import { open } from "@tauri-apps/plugin-dialog"
import { useMemo, useRef, useState } from "react"
import { toast } from "sonner"

import { commands } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { useLocalRepos } from "@/hooks/useLocalRepos"

export function LocalRepos() {
  const navigate = useNavigate()
  const [filter, setFilter] = useState("")
  const inputRef = useRef<HTMLInputElement>(null)
  const cardRef = useRef<HTMLDivElement>(null)

  const { data } = useLocalRepos()

  const filteredRepoDirs = useMemo(() => {
    if (!data) return []
    if (!filter) return data
    const lowerFilter = filter.toLowerCase()
    return data.filter((dir) => dir.toLowerCase().includes(lowerFilter))
  }, [data, filter])

  useHotkey("/", () => inputRef.current?.focus(), {
    enabled: document.activeElement !== inputRef.current,
    target: cardRef,
  })

  useHotkey("Escape", () => inputRef.current?.blur(), {
    target: inputRef,
  })

  const handleRowKeyDown = (
    e: React.KeyboardEvent<HTMLTableRowElement>,
    dir: string,
  ) => {
    if (e.key === "Enter") {
      navigate({ to: "/localRepo/$dir", params: { dir } })
    }
  }

  const handleOpenLocalRepo = async () => {
    const directory = await open({
      title: "Select a local repository",
      directory: true,
    })
    if (directory == null) return
    const result = await commands.getJjStatus(directory)
    if (result.status === "error") {
      console.error("Unexpected error validating repository:", result.error)
      toast("Something went wrong", {
        className: "bg-destructive",
      })
      return
    }
    if (result.data.isJjRepo) {
      navigate({ to: "/localRepo/$dir", params: { dir: directory } })
    } else {
      toast("Selected directory is not a valid repository", {
        className: "bg-destructive",
      })
    }
  }

  return (
    <Card ref={cardRef} className="p-4">
      <CardTitle>Local Repositories</CardTitle>
      <CardContent className="mt-5">
        <div className="flex gap-4">
          <Input
            ref={inputRef}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter repositories..."
            className="mb-4 grow"
          />
          <Button className="w-20" onClick={handleOpenLocalRepo}>
            Open
          </Button>
        </div>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Path</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredRepoDirs.map((dir) => (
              <TableRow
                key={dir}
                tabIndex={0}
                onKeyDown={(e) => handleRowKeyDown(e, dir)}
                className="focus:outline-none focus:bg-muted/50 cursor-pointer"
              >
                <TableCell>
                  <Link
                    to="/localRepo/$dir"
                    params={{ dir }}
                    className="underline"
                    tabIndex={-1}
                  >
                    {dir}
                  </Link>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  )
}
