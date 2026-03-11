import { Link, Outlet, useLocation } from "react-router"
import {
  LayoutDashboard,
  FileCode,
  Layers,
  FolderKanban,
  Bot,
  ScrollText,
  Settings,
  Sun,
  Moon,
  Monitor,
} from "lucide-react"
import { useTheme } from "@/lib/theme"
import { cn } from "@/lib/utils"

const navItems = [
  { path: "/", label: "Dashboard", icon: LayoutDashboard },
  { path: "/skills", label: "Skills", icon: FileCode },
  { path: "/profiles", label: "Profiles", icon: Layers },
  { path: "/projects", label: "Projects", icon: FolderKanban },
  { path: "/agents", label: "Agent Tools", icon: Bot },
  { path: "/activity", label: "Activity Log", icon: ScrollText },
  { path: "/settings", label: "Settings", icon: Settings },
]

const themeOptions = [
  { value: "light" as const, icon: Sun, label: "Light" },
  { value: "dark" as const, icon: Moon, label: "Dark" },
  { value: "system" as const, icon: Monitor, label: "System" },
]

export function AppLayout() {
  const location = useLocation()
  const { theme, setTheme } = useTheme()

  return (
    <div className="flex h-screen bg-background text-foreground">
      {/* Sidebar */}
      <nav className="flex w-60 flex-col border-r border-border bg-card p-4 pt-5">
        {/* Brand */}
        <div className="mb-5 flex items-center gap-2.5 px-1">
          <Layers className="h-6 w-6 text-primary" />
          <span className="text-base font-bold">Skills Manager</span>
        </div>

        {/* Nav items */}
        <ul className="flex-1 space-y-1">
          {navItems.map((item) => {
            const Icon = item.icon
            const isActive = location.pathname === item.path
            return (
              <li key={item.path}>
                <Link
                  to={item.path}
                  className={cn(
                    "flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition-all duration-150",
                    isActive
                      ? "bg-primary text-primary-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-muted hover:text-foreground"
                  )}
                >
                  <Icon className="h-4 w-4" />
                  {item.label}
                </Link>
              </li>
            )
          })}
        </ul>

        {/* Theme switcher */}
        <div className="border-t border-border pt-3">
          <div className="flex items-center justify-between rounded-md bg-muted p-1">
            {themeOptions.map((opt) => {
              const Icon = opt.icon
              return (
                <button
                  key={opt.value}
                  onClick={() => setTheme(opt.value)}
                  title={opt.label}
                  className={cn(
                    "flex-1 rounded-sm p-1.5 transition-colors",
                    theme === opt.value
                      ? "bg-background text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground"
                  )}
                >
                  <Icon className="mx-auto h-4 w-4" />
                </button>
              )
            })}
          </div>
        </div>
      </nav>

      {/* Main area */}
      <main className="flex-1 overflow-auto p-8">
        <div className="animate-page-in">
          <Outlet />
        </div>
      </main>
    </div>
  )
}
