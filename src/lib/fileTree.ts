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

  return compactTree(sortTree(root.children))
}

export function compareFilePaths<T>(
  getFilePath: (file: T) => string,
): (a: T, b: T) => number {
  return (a, b) => {
    const pathA = getFilePath(a)
    const pathB = getFilePath(b)

    const partsA = pathA.split("/")
    const partsB = pathB.split("/")

    function compareParts(parts1: string[], parts2: string[]): number {
      const part1 = parts1[0]
      const part2 = parts2[0]

      if (!part1 || !part2) {
        console.warn(
          "One of the file paths is empty. This may indicate an issue with the getFilePath function.",
          { pathA, pathB },
        )
      }
      // Both paths exhausted
      if (!part1 && !part2) return 0
      if (!part1) return -1
      if (!part2) return 1

      const hasMoreParts1 = parts1.length > 1
      const hasMoreParts2 = parts2.length > 1

      // If one is a directory and one is a file
      if (hasMoreParts1 && !hasMoreParts2) return -1 // dir comes first
      if (!hasMoreParts1 && hasMoreParts2) return 1 // file comes second

      // Both are directories or both are files, compare names
      const nameCmp = part1.localeCompare(part2)
      if (nameCmp !== 0) return nameCmp

      // Names are the same, recurse to next level
      return compareParts(parts1.slice(1), parts2.slice(1))
    }

    return compareParts(partsA, partsB)
  }
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

function compactTree<T>(nodes: TreeNode<T>[]): TreeNode<T>[] {
  return nodes.map((node) => {
    if (node.type === "file") return node

    let name = node.name
    let current = node
    while (current.children.length === 1) {
      const child = current.children[0]
      if (child.type !== "directory") break
      name = `${name}/${child.name}`
      current = child
    }

    return {
      ...current,
      name,
      children: compactTree(current.children),
    }
  })
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
