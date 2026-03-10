import { Link, Outlet, useLocation } from "react-router"
import {
  LayoutDashboard,
  Wrench,
  Layers,
  Bot,
  Activity,
  Settings,
} from "lucide-react"

const navItems = [
  { path: "/", label: "Dashboard", icon: LayoutDashboard },
  { path: "/skills", label: "Skills", icon: Wrench },
  { path: "/profiles", label: "Profiles", icon: Layers },
  { path: "/agents", label: "Agents", icon: Bot },
  { path: "/activity", label: "Activity", icon: Activity },
  { path: "/settings", label: "Settings", icon: Settings },
]

export function AppLayout() {
  const location = useLocation()

  return (
    <div className="flex h-screen bg-background text-foreground">
      <nav className="w-56 border-r border-border bg-muted/40 p-4">
        <h1 className="mb-6 text-lg font-semibold">Skills Manager</h1>
        <ul className="space-y-1">
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
      </nav>
      <main className="flex-1 overflow-auto p-6">
        <Outlet />
      </main>
    </div>
  )
}
