import { useQuery } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { listSkills, getRecentLogs } from "@/lib/api"

export function Dashboard() {
  const skills = useQuery({ queryKey: ["skills"], queryFn: listSkills })
  const logs = useQuery({ queryKey: ["logs"], queryFn: () => getRecentLogs(5) })

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Dashboard</h2>
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader><CardTitle>Skills</CardTitle></CardHeader>
          <CardContent>
            <p className="text-3xl font-bold">{skills.data?.length ?? 0}</p>
          </CardContent>
        </Card>
      </div>
      <Card>
        <CardHeader><CardTitle>Recent Activity</CardTitle></CardHeader>
        <CardContent>
          {logs.data?.map((log) => (
            <div key={log.id} className="flex justify-between border-b border-border py-2 text-sm">
              <span>{log.operation} ({log.source})</span>
              <span className="text-muted-foreground">{log.timestamp}</span>
            </div>
          ))}
          {(!logs.data || logs.data.length === 0) && (
            <p className="text-sm text-muted-foreground">No recent activity</p>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
