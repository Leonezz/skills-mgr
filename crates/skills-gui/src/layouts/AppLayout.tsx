import { Link, Outlet, useLocation } from "react-router"
import {
  LayoutDashboard,
  Wrench,
  Layers,
  Bot,
  Activity,
  Settings,
  Sun,
  Moon,
  Monitor,
} from "lucide-react"
import { useTheme } from "@/lib/theme"

const navItems = [
  { path: "/", label: "Dashboard", icon: LayoutDashboard },
  { path: "/skills", label: "Skills", icon: Wrench },
  { path: "/profiles", label: "Profiles", icon: Layers },
  { path: "/agents", label: "Agents", icon: Bot },
  { path: "/activity", label: "Activity", icon: Activity },
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
      <nav className="flex w-56 flex-col border-r border-border bg-muted/40 p-4">
        <h1 className="mb-6 text-lg font-semibold">Skills Manager</h1>
        <ul className="flex-1 space-y-1">
          {navItems.map((item) => {
            const Icon = item.icon
            const isActive = location.pathname === item.path
            return (
              <li key={item.path}>
                <Link
                  to={item.path}
                  className={`flex items-center gap-2 rounded-md px-3 py-2 text-sm ${
                    isActive
                      ? "bg-primary text-primary-foreground"
                      : "hover:bg-muted text-muted-foreground"
                  }`}
                >
                  <Icon className="h-4 w-4" />
                  {item.label}
                </Link>
              </li>
            )
          })}
        </ul>
        <div className="border-t border-border pt-3">
          <div className="flex items-center justify-between rounded-md bg-muted p-1">
            {themeOptions.map((opt) => {
              const Icon = opt.icon
              return (
                <button
                  key={opt.value}
                  onClick={() => setTheme(opt.value)}
                  title={opt.label}
                  className={`flex-1 rounded-sm p-1.5 transition-colors ${
                    theme === opt.value
                      ? "bg-background text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground"
                  }`}
                >
                  <Icon className="mx-auto h-4 w-4" />
                </button>
              )
            })}
          </div>
        </div>
      </nav>
      <main className="flex-1 overflow-auto p-6">
        <Outlet />
      </main>
    </div>
  )
}
