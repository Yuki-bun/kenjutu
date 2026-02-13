import { openUrl } from "@tauri-apps/plugin-opener"
import Markdown from "react-markdown"
import rehypeRaw from "rehype-raw"
import remarkGfm from "remark-gfm"

import { cn } from "@/lib/utils"

import styles from "./MarkdownContent.module.css"

type MarkdownContentProps = {
  children: string
  className?: string
}

export function MarkdownContent({ children, className }: MarkdownContentProps) {
  const handleLinkClick = (event: React.MouseEvent<HTMLAnchorElement>) => {
    event.preventDefault()
    const href = event.currentTarget.href
    if (href) {
      openUrl(href).catch((err: unknown) => {
        console.error("Failed to open URL:", err)
      })
    }
  }

  return (
    <div className={cn(styles.markdownContent, className)}>
      <Markdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeRaw]}
        components={{
          a: (props) => <a {...props} onClick={handleLinkClick} />,
        }}
      >
        {children}
      </Markdown>
    </div>
  )
}
