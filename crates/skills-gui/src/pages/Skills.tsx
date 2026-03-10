import { useQuery } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { listSkills } from "@/lib/api"

export function Skills() {
  const { data: skills, isLoading } = useQuery({ queryKey: ["skills"], queryFn: listSkills })

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Skills Registry</h2>
      </div>
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : skills && skills.length > 0 ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {skills.map((skill) => (
            <Card key={skill.name}>
              <CardHeader>
                <CardTitle className="text-base">{skill.name}</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">
                  {skill.description ?? "No description"}
                </p>
                <div className="mt-2 flex gap-1">
                  {skill.source_type && (
                    <span className="inline-block rounded-md bg-secondary px-2 py-0.5 text-xs text-secondary-foreground">
                      {skill.source_type}
                    </span>
                  )}
                  <span className="inline-block rounded-md bg-secondary px-2 py-0.5 text-xs text-secondary-foreground">
                    {skill.files.length} files
                  </span>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <p className="text-muted-foreground">No skills in registry.</p>
      )}
    </div>
  )
}
