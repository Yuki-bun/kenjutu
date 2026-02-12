import Markdown from "react-markdown"
import remarkGfm from "remark-gfm"

import styles from "./MarkdownContent.module.css"

type MarkdownContentProps = {
  children: string
  className?: string
}

export function MarkdownContent({ children, className }: MarkdownContentProps) {
  return (
    <div
      className={
        className
          ? `${styles.markdownContent} ${className}`
          : styles.markdownContent
      }
    >
      <Markdown remarkPlugins={[remarkGfm]}>{children}</Markdown>
    </div>
  )
}
