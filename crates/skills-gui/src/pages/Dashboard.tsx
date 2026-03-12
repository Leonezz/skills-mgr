import { useQuery } from "@tanstack/react-query"
import { Link } from "react-router"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { listSkills, listProfiles, listAgents, listProjects, getRecentLogs } from "@/lib/api"
import { Wrench, Layers, FolderKanban, Bot, Plus } from "lucide-react"
import type { LogEntry } from "@/lib/schemas"

const operationColors: Record<string, string> = {
  profile_activate: "bg-emerald-500",
  skill_create: "bg-indigo-500",
  skill_import: "bg-indigo-500",
  profile_deactivate: "bg-orange-500",
  profile_create: "bg-indigo-500",
  skill_remove: "bg-red-500",
  profile_delete: "bg-red-500",
  agent_add: "bg-indigo-500",
  agent_edit: "bg-orange-500",
  agent_remove: "bg-red-500",
  project_add: "bg-indigo-500",
  project_remove: "bg-red-500",
}

function ActivityRow({ log }: { log: LogEntry }) {
  const dotColor = operationColors[log.operation] ?? "bg-muted-foreground"

  return (
    <div className="flex items-center gap-3 py-2.5 border-b border-border last:border-0">
      <span className={`h-2 w-2 shrink-0 rounded-full ${dotColor}`} />
      <span className="flex-1 truncate text-sm">{log.details ?? log.operation}</span>
      <span className="shrink-0 text-xs text-muted-foreground">{log.timestamp}</span>
    </div>
  )
}

export function Dashboard() {
  const skills = useQuery({ queryKey: ["skills"], queryFn: listSkills })
  const profiles = useQuery({ queryKey: ["profiles"], queryFn: listProfiles })
  const projects = useQuery({ queryKey: ["projects"], queryFn: listProjects })
  const agents = useQuery({ queryKey: ["agents"], queryFn: listAgents })
  const logs = useQuery({ queryKey: ["logs"], queryFn: () => getRecentLogs(5) })

  const profileCount = profiles.data?.profiles?.length ?? 0
  const projectCount = projects.data?.length ?? 0

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* Header — fixed */}
      <div className="shrink-0 flex items-center justify-between pb-7">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Dashboard</h2>
          <p className="text-sm text-muted-foreground">
            Overview of your skill management workspace
          </p>
        </div>
        <div className="flex gap-2.5">
          <Button asChild>
            <Link to="/projects">
              <FolderKanban className="h-4 w-4" />
              Manage Projects
            </Link>
          </Button>
          <Button variant="outline" asChild>
            <Link to="/skills">
              <Plus className="h-4 w-4" />
              Add Skill
            </Link>
          </Button>
        </div>
      </div>

      {/* Body — scrollable */}
      <div className="flex-1 min-h-0 overflow-y-auto space-y-7">

      {/* Stat Cards */}
      <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
        <Card className="animate-list-item" style={{ animationDelay: "0ms" }}>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Skills
            </CardTitle>
            <Wrench className="h-4 w-4 shrink-0 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <p className="text-3xl font-bold">{skills.data?.length ?? 0}</p>
            <p className="text-xs text-muted-foreground">In registry</p>
          </CardContent>
        </Card>
        <Card className="animate-list-item" style={{ animationDelay: "60ms" }}>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Profiles
            </CardTitle>
            <Layers className="h-4 w-4 shrink-0 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <p className="text-3xl font-bold">{profileCount}</p>
            <p className="text-xs text-muted-foreground">Defined</p>
          </CardContent>
        </Card>
        <Card className="animate-list-item" style={{ animationDelay: "120ms" }}>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Projects
            </CardTitle>
            <FolderKanban className="h-4 w-4 shrink-0 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <p className="text-3xl font-bold">{projectCount}</p>
            <p className="text-xs text-muted-foreground">Registered</p>
          </CardContent>
        </Card>
        <Card className="animate-list-item" style={{ animationDelay: "180ms" }}>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Agent Tools
            </CardTitle>
            <Bot className="h-4 w-4 shrink-0 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <p className="text-3xl font-bold">{agents.data?.length ?? 0}</p>
            <p className="text-xs text-muted-foreground">Configured</p>
          </CardContent>
        </Card>
      </div>

      {/* Recent Projects */}
      <Card>
        <CardContent className="p-6">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-base font-semibold">Projects</h3>
            <Link
              to="/projects"
              className="text-xs font-medium text-primary hover:underline"
            >
              Manage projects
            </Link>
          </div>
          {projects.data && projects.data.length > 0 ? (
            <div className="space-y-3">
              {projects.data.slice(0, 3).map((project) => (
                <div key={project.path} className="flex items-center gap-3">
                  <FolderKanban className="h-4 w-4 shrink-0 text-muted-foreground" />
                  <span className="text-sm font-medium truncate flex-1">{project.name}</span>
                  <div className="flex gap-1.5 shrink-0">
                    {project.active_profiles.map((p) => (
                      <Badge key={p} variant="secondary" className="text-[10px]">{p}</Badge>
                    ))}
                    {project.active_profiles.length === 0 && (
                      <span className="text-xs text-muted-foreground">No active profiles</span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No projects registered yet.</p>
          )}
        </CardContent>
      </Card>

      {/* Recent Activity */}
      <Card>
        <CardContent className="p-6">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-base font-semibold">Recent Activity</h3>
            <Link
              to="/activity"
              className="text-xs font-medium text-primary hover:underline"
            >
              View all
            </Link>
          </div>
          {logs.data && logs.data.length > 0 ? (
            <div>
              {logs.data.map((log) => (
                <ActivityRow key={log.id} log={log} />
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No recent activity</p>
          )}
        </CardContent>
      </Card>
      </div>
    </div>
  )
}
