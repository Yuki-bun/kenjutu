import { Link } from "@tanstack/react-router"

import { AppCommands } from "./AppCommands"
import { DeviceAuth } from "./DeviceAuth"

export function AppHeader() {
  return (
    <header className="z-50 w-full shrink-0 border-b">
      <nav className="w-full flex h-14 items-center justify-between px-4">
        {/* @ts-expect-error index route "/" not in generated types */}
        <Link to={"/"}>
          <div className="font-semibold text-lg">Revue</div>
        </Link>
        <AppCommands />
        <DeviceAuth />
      </nav>
    </header>
  )
}
