import { Link } from "@tanstack/react-router"

import { AppCommands } from "./AppCommands"
import { DeviceAuth } from "./DeviceAuth"

export function AppHeader() {
  return (
    <header className="z-50 w-full shrink-0 border-b">
      <nav className="w-full flex h-10 items-center justify-between px-4 gap-2">
        {/* @ts-expect-error index route "/" not in generated types */}
        <Link to={"/"}>
          <div className="font-semibold text-lg">Kenjutu</div>
        </Link>
        <div className="flex-1" />
        <AppCommands />
        <DeviceAuth />
      </nav>
    </header>
  )
}
