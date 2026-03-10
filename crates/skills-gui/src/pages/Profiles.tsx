import { useQuery } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { listProfiles } from "@/lib/api"

export function Profiles() {
  const { data, isLoading } = useQuery({ queryKey: ["profiles"], queryFn: listProfiles })

  const profiles = data as { base?: { skills?: string[] }; profiles?: Record<string, { description?: string; skills?: string[]; includes?: string[] }> } | undefined

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Profiles</h2>
      {profiles?.base?.skills && profiles.base.skills.length > 0 && (
        <Card>
          <CardHeader><CardTitle>Base Skills</CardTitle></CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-2">
              {profiles.base.skills.map((s: string) => (
                <span key={s} className="rounded-md bg-secondary px-2 py-1 text-sm">{s}</span>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : profiles?.profiles && Object.keys(profiles.profiles).length > 0 ? (
        <div className="space-y-3">
          {Object.entries(profiles.profiles).map(([name, profile]) => (
            <Card key={name}>
              <CardHeader>
                <CardTitle className="text-base">{name}</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">{profile.description ?? ""}</p>
                <div className="mt-2 flex flex-wrap gap-1">
                  {profile.skills?.map((s: string) => (
                    <span key={s} className="rounded-md bg-secondary px-2 py-0.5 text-xs">{s}</span>
                  ))}
                </div>
                {profile.includes && profile.includes.length > 0 && (
                  <p className="mt-1 text-xs text-muted-foreground">Includes: {profile.includes.join(", ")}</p>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <p className="text-muted-foreground">No profiles defined.</p>
      )}
    </div>
  )
}
