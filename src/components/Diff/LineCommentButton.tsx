import { MessageSquarePlus } from "lucide-react"

export function LineCommentButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation()
        onClick()
      }}
      className="absolute left-0 top-1/2 -translate-y-1/2 opacity-0 group-hover/line:opacity-100 transition-opacity bg-blue-500 text-white rounded-sm p-0.5 hover:bg-blue-600 z-10"
    >
      <MessageSquarePlus className="w-3 h-3" />
    </button>
  )
}
