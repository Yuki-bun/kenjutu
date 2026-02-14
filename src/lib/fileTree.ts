export type TreeNode<T> = FileNode<T> | DirectoryNode<T>

export type FileNode<T> = {
  type: "file"
  name: string
  path: string
  file: T
}

export type DirectoryNode<T> = {
  type: "directory"
  name: string
  path: string
  children: TreeNode<T>[]
}

export function buildFileTree<T>(
  files: T[],
  getFilePath: (file: T) => string,
): TreeNode<T>[] {
  const root: DirectoryNode<T> = {
    type: "directory",
    name: "",
    path: "",
    children: [],
  }

  for (const file of files) {
    const filePath = getFilePath(file)
    if (!filePath) continue

    const parts = filePath.split("/")
    insertIntoTree(root, parts, file, getFilePath)
  }

  return sortTree(root.children)
}

export function sortFilesInTreeOrder<T>(
  files: T[],
  getFilePath: (file: T) => string,
): T[] {
  const tree = buildFileTree(files, getFilePath)
  const sortedFiles: T[] = []

  function traverse(nodes: TreeNode<T>[]) {
    for (const node of nodes) {
      if (node.type === "file") {
        sortedFiles.push(node.file)
      }
      if (node.type === "directory") {
        traverse(node.children)
      }
    }
  }

  traverse(tree)
  return sortedFiles
}

function insertIntoTree<T>(
  parent: DirectoryNode<T>,
  pathParts: string[],
  file: T,
  getFilePath: (file: T) => string,
): void {
  if (pathParts.length === 1) {
    const fileNode: FileNode<T> = {
      type: "file",
      name: pathParts[0],
      path: getFilePath(file),
      file,
    }
    parent.children.push(fileNode)
  } else {
    const [dirName, ...rest] = pathParts
    const dirNode = parent.children
      .filter((child) => child.type === "directory")
      .find((dir) => dir.name === dirName)

    if (dirNode) {
      insertIntoTree(dirNode, rest, file, getFilePath)
      return
    }
    const newDirNode = {
      type: "directory" as const,
      name: dirName,
      path: parent.path ? `${parent.path}/${dirName}` : dirName,
      children: [],
    }
    parent.children.push(newDirNode)
    insertIntoTree(newDirNode, rest, file, getFilePath)
  }
}

function sortTree<T>(nodes: TreeNode<T>[]): TreeNode<T>[] {
  const sorted = [...nodes].sort((a, b) => {
    // Directories first
    if (a.type === "directory" && b.type === "file") return -1
    if (a.type === "file" && b.type === "directory") return 1

    return a.name.localeCompare(b.name)
  })

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
