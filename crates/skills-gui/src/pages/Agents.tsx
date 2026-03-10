import { useQuery } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { listAgents } from "@/lib/api"

export function Agents() {
  const { data, isLoading } = useQuery({ queryKey: ["agents"], queryFn: listAgents })

  const agents = data as { agents?: Record<string, { project_path: string; global_path: string }> } | undefined

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Agents</h2>
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : agents?.agents && Object.keys(agents.agents).length > 0 ? (
        <div className="space-y-3">
          {Object.entries(agents.agents).map(([name, def]) => (
            <Card key={name}>
              <CardHeader>
                <CardTitle className="text-base">{name}</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">Project: {def.project_path}</p>
                <p className="text-sm text-muted-foreground">Global: {def.global_path}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <p className="text-muted-foreground">No agents configured.</p>
      )}
    </div>
  )
}
