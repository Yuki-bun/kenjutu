import { User } from "lucide-react"

type UserInfo = {
  login?: string
  avatar_url?: string
}

export function UserAvatar({ user }: { user?: UserInfo | null }) {
  return (
    <div className="w-10 h-10 rounded-full bg-muted flex items-center justify-center text-sm font-medium shrink-0 overflow-hidden">
      {user?.avatar_url ? (
        <img
          src={user.avatar_url}
          alt={user.login}
          className="w-full h-full object-cover"
        />
      ) : (
        <User className="w-5 h-5 text-muted-foreground" />
      )}
    </div>
  )
}
