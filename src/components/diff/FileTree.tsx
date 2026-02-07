import * as Collapsible from "@radix-ui/react-collapsible"
import { ChevronDown, ChevronRight, Folder, FolderOpen } from "lucide-react"
import { useState } from "react"

import { commands, FileChangeStatus, FileEntry } from "@/bindings"
import { ErrorDisplay } from "@/components/error"
import { useFailableQuery } from "@/hooks/useRpcQuery"
import { cn } from "@/lib/utils"

import { ScrollFocus, useScrollFocusItem } from "../ScrollFocus"

type DirectoryNode = {
  type: "directory"
  name: string
  path: string
  children: TreeNode[]
}

type FileNode = {
  type: "file"
  name: string
  path: string
  fileEntry: FileEntry
}

type TreeNode = DirectoryNode | FileNode

type FileTreeProps = {
  localDir: string
  commitSha: string | undefined
}

export function FileTree({ localDir, commitSha }: FileTreeProps) {
  const { data, error, isLoading } = useFailableQuery({
    queryKey: ["commit-file-list", localDir, commitSha],
    queryFn: () => {
      if (!commitSha) {
        throw new Error("No commit selected")
      }
      return commands.getCommitFileList(localDir, commitSha)
    },
    enabled: !!commitSha,
  })

  if (!commitSha) {
    return (
      <div className="px-2 py-3">
        <p className="text-xs text-muted-foreground">
          Select a commit to view files
        </p>
      </div>
    )
  }

  if (isLoading) {
    return (
      <div className="px-2 py-3">
        <h3 className="text-xs font-medium text-muted-foreground mb-2">
          Files Changed
        </h3>
        <p className="text-xs text-muted-foreground">Loading files...</p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="px-2 py-3">
        <h3 className="text-xs font-medium text-muted-foreground mb-2">
          Files Changed
        </h3>
        <ErrorDisplay error={error} />
      </div>
    )
  }

  if (!data || data.files.length === 0) {
    return (
      <div className="px-2 py-3">
        <h3 className="text-xs font-medium text-muted-foreground mb-2">
          Files Changed
        </h3>
        <p className="text-xs text-muted-foreground">No files changed</p>
      </div>
    )
  }

  const tree = buildFileTree(data.files)

  return (
    <div className="px-2 py-3">
      <h3 className="text-xs font-medium text-muted-foreground mb-2">
        Files Changed ({data.files.length})
      </h3>
      <ScrollFocus className="space-y-0.5">
        {tree.map((node) => (
          <TreeNodeComponent key={node.path} node={node} depth={0} />
        ))}
      </ScrollFocus>
    </div>
  )
}

function TreeNodeComponent({ node, depth }: { node: TreeNode; depth: number }) {
  const [isOpen, setIsOpen] = useState(true)

  if (node.type === "directory") {
    return (
      <Collapsible.Root open={isOpen} onOpenChange={setIsOpen}>
        <DirectoryRow
          node={node}
          depth={depth}
          isOpen={isOpen}
          onToggle={() => setIsOpen(!isOpen)}
        />
        <Collapsible.Content>
          <div>
            {node.children.map((child) => (
              <TreeNodeComponent
                key={child.path}
                node={child}
                depth={depth + 1}
              />
            ))}
          </div>
        </Collapsible.Content>
      </Collapsible.Root>
    )
  } else {
    return <FileRow node={node} depth={depth} />
  }
}

function DirectoryRow({
  node,
  depth,
  isOpen,
  onToggle,
}: {
  node: DirectoryNode
  depth: number
  isOpen: boolean
  onToggle: () => void
}) {
  const { ref } = useScrollFocusItem<HTMLButtonElement>(node.path)

  return (
    <Collapsible.Trigger asChild>
      <button
        ref={ref}
        onClick={onToggle}
        className="flex items-center gap-1.5 w-full text-left py-0.5 px-1 rounded hover:bg-muted/50 cursor-pointer"
        style={{ paddingLeft: `${depth * 12 + 4}px` }}
      >
        {isOpen ? (
          <ChevronDown className="w-3 h-3 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="w-3 h-3 text-muted-foreground shrink-0" />
        )}
        {isOpen ? (
          <FolderOpen className="w-3 h-3 text-muted-foreground shrink-0" />
        ) : (
          <Folder className="w-3 h-3 text-muted-foreground shrink-0" />
        )}
        <span className="text-xs font-medium truncate">{node.name}</span>
      </button>
    </Collapsible.Trigger>
  )
}

function FileRow({ node, depth }: { node: FileNode; depth: number }) {
  const { fileEntry } = node
  const statusIndicator = getStatusIndicator(fileEntry.status)
  const { ref } = useScrollFocusItem<HTMLDivElement>(node.path)

  return (
    <div
      className="flex items-center gap-1.5 py-0.5 px-1 rounded hover:bg-muted/50"
      style={{ paddingLeft: `${depth * 12 + 4}px` }}
      ref={ref}
      tabIndex={0}
    >
      <div className="w-3 h-3 shrink-0" /> {/* Spacer for alignment */}
      <span
        className={cn(
          "w-3 h-3 shrink-0 text-[10px] font-bold flex items-center justify-center",
          statusIndicator.color,
        )}
      >
        {statusIndicator.letter}
      </span>
      <span className="text-xs truncate flex-1">{node.name}</span>
      <div className="flex items-center gap-1 shrink-0">
        {!fileEntry.isBinary && (
          <span className="text-[10px] text-muted-foreground whitespace-nowrap">
            <span className="text-green-600">+{fileEntry.additions}</span>{" "}
            <span className="text-red-600">-{fileEntry.deletions}</span>
          </span>
        )}
      </div>
    </div>
  )
}

function getStatusIndicator(status: FileChangeStatus): {
  letter: string
  color: string
} {
  switch (status) {
    case "added":
      return {
        letter: "A",
        color: "text-green-600 dark:text-green-400",
      }
    case "modified":
      return {
        letter: "M",
        color: "text-blue-600 dark:text-blue-400",
      }
    case "deleted":
      return {
        letter: "D",
        color: "text-red-600 dark:text-red-400",
      }
    case "renamed":
      return {
        letter: "R",
        color: "text-purple-600 dark:text-purple-400",
      }
    case "copied":
      return {
        letter: "C",
        color: "text-yellow-600 dark:text-yellow-400",
      }
    case "typechange":
      return {
        letter: "T",
        color: "text-orange-600 dark:text-orange-400",
      }
    default:
      return {
        letter: "?",
        color: "text-gray-600 dark:text-gray-400",
      }
  }
}

function buildFileTree(files: FileEntry[]): TreeNode[] {
  const root: DirectoryNode = {
    type: "directory",
    name: "",
    path: "",
    children: [],
  }

  for (const file of files) {
    const filePath = file.newPath || file.oldPath
    if (!filePath) continue

    const parts = filePath.split("/")
    insertIntoTree(root, parts, file)
  }

  return sortTree(root.children)
}

function insertIntoTree(
  parent: DirectoryNode,
  pathParts: string[],
  fileEntry: FileEntry,
): void {
  if (pathParts.length === 1) {
    const fileNode: FileNode = {
      type: "file",
      name: pathParts[0],
      path: fileEntry.newPath || fileEntry.oldPath || "",
      fileEntry,
    }
    parent.children.push(fileNode)
  } else {
    const [dirName, ...rest] = pathParts
    const dirNode = parent.children
      .filter((child) => child.type === "directory")
      .find((dir) => dir.name === dirName)

    if (dirNode) {
      insertIntoTree(dirNode, rest, fileEntry)
      return
    }
    const newDirNode = {
      type: "directory" as const,
      name: dirName,
      path: parent.path ? `${parent.path}/${dirName}` : dirName,
      children: [],
    }
    parent.children.push(newDirNode)
    insertIntoTree(newDirNode, rest, fileEntry)
  }
}

function sortTree(nodes: TreeNode[]): TreeNode[] {
  const sorted = [...nodes].sort((a, b) => {
    // Directories first
    if (a.type === "directory" && b.type === "file") return -1
    if (a.type === "file" && b.type === "directory") return 1

    // Then alphabetically
    return a.name.localeCompare(b.name)
  })

  // Recursively sort children of directories
  return sorted.map((node) => {
    if (node.type === "directory") {
      return {
        ...node,
        children: sortTree(node.children),
      }
    }
    return node
  })
}
