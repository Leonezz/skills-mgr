import { useQuery } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { listSkills, listProfiles, listAgents, getRecentLogs } from "@/lib/api"

export function Dashboard() {
  const skills = useQuery({ queryKey: ["skills"], queryFn: listSkills })
  const profiles = useQuery({ queryKey: ["profiles"], queryFn: listProfiles })
  const agents = useQuery({ queryKey: ["agents"], queryFn: listAgents })
  const logs = useQuery({ queryKey: ["logs"], queryFn: () => getRecentLogs(5) })

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Dashboard</h2>
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader><CardTitle>Skills</CardTitle></CardHeader>
          <CardContent>
            <p className="text-3xl font-bold">{skills.data?.length ?? 0}</p>
            <p className="text-sm text-muted-foreground">in registry</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader><CardTitle>Profiles</CardTitle></CardHeader>
          <CardContent>
            <p className="text-3xl font-bold">{profiles.data?.profiles?.length ?? 0}</p>
            <p className="text-sm text-muted-foreground">defined</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader><CardTitle>Agents</CardTitle></CardHeader>
          <CardContent>
            <p className="text-3xl font-bold">{agents.data?.length ?? 0}</p>
            <p className="text-sm text-muted-foreground">configured</p>
          </CardContent>
        </Card>
      </div>
      <Card>
        <CardHeader><CardTitle>Recent Activity</CardTitle></CardHeader>
        <CardContent>
          {logs.data && logs.data.length > 0 ? (
            logs.data.map((log) => (
              <div key={log.id} className="flex justify-between border-b border-border py-2 text-sm">
                <span>
                  <span className="font-medium">{log.operation}</span>
                  <span className="ml-2 text-muted-foreground">({log.source})</span>
                  {log.details && <span className="ml-2 text-muted-foreground">{log.details}</span>}
                </span>
                <span className="text-muted-foreground">{log.timestamp}</span>
              </div>
            ))
          ) : (
            <p className="text-sm text-muted-foreground">No recent activity</p>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
