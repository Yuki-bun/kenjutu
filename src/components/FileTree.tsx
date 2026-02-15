import {
  Check,
  ChevronDown,
  ChevronRight,
  Circle,
  Folder,
  FolderOpen,
} from "lucide-react"
import { useState } from "react"

import { FileChangeStatus, FileEntry } from "@/bindings"
import { ErrorDisplay } from "@/components/error"
import { Collapsible, CollapsibleContent } from "@/components/ui/collapsible"
import { useCommitFileList } from "@/hooks/useCommitFileList"
import {
  buildFileTree,
  DirectoryNode as TDirectoryNode,
  FileNode as TFileNode,
  TreeNode as TTreeNode,
} from "@/lib/fileTree"
import { cn } from "@/lib/utils"

import {
  focusItemInPanel,
  PANEL_KEYS,
  ScrollFocus,
  useScrollFocusItem,
} from "./ScrollFocus"

type TreeNode = TTreeNode<FileEntry>
type DirectoryNode = TDirectoryNode<FileEntry>
type FileNode = TFileNode<FileEntry>

type FileTreeProps = {
  localDir: string
  commitSha: string | undefined
}

export function FileTree({ localDir, commitSha }: FileTreeProps) {
  const { data, error, isLoading } = useCommitFileList(localDir, commitSha)

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

  const tree = buildFileTree(
    data.files,
    (file) => file.newPath || file.oldPath || "",
  )

  return (
    <div className="px-2 py-3">
      <h3 className="text-xs font-medium text-muted-foreground mb-2">
        Files Changed ({data.files.length})
      </h3>
      <ScrollFocus className="space-y-0.5" panelKey={PANEL_KEYS.fileTree}>
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
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        <DirectoryRow
          node={node}
          depth={depth}
          isOpen={isOpen}
          onToggle={() => setIsOpen(!isOpen)}
        />
        <CollapsibleContent>
          <div>
            {node.children.map((child) => (
              <TreeNodeComponent
                key={child.path}
                node={child}
                depth={depth + 1}
              />
            ))}
          </div>
        </CollapsibleContent>
      </Collapsible>
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
    <Collapsible asChild>
      <button
        ref={ref}
        onClick={onToggle}
        className="flex items-center gap-1.5 w-full text-left py-0.5 px-1 rounded hover:bg-muted/50 cursor-pointer focusKey"
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
    </Collapsible>
  )
}

function FileRow({ node, depth }: { node: FileNode; depth: number }) {
  const { file } = node
  const statusIndicator = getStatusIndicator(file.status)
  const { ref } = useScrollFocusItem<HTMLButtonElement>(node.path)

  return (
    <button
      className="flex items-center gap-1.5 py-0.5 px-1 rounded data-[focused=true]:bg-accent/50 w-full text-left focusKey"
      style={{ paddingLeft: `${depth * 12 + 4}px` }}
      ref={ref}
      tabIndex={0}
      onClick={() => focusItemInPanel(PANEL_KEYS.diffVew, node.path)}
    >
      <div className="w-3 h-3 shrink-0" /> {/* Spacer for alignment */}
      {file.isReviewed ? (
        <Check className="w-3 h-3 shrink-0 text-green-600 dark:text-green-400" />
      ) : (
        <Circle className="w-3 h-3 shrink-0 text-muted-foreground" />
      )}
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
        {!file.isBinary && (
          <span className="text-[10px] text-muted-foreground whitespace-nowrap">
            <span className="text-green-600">+{file.additions}</span>{" "}
            <span className="text-red-600">-{file.deletions}</span>
          </span>
        )}
      </div>
    </button>
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
