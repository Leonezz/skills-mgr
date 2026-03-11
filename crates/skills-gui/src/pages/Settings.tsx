import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"

export function Settings() {
  const [basePath] = useState("~/.skills-mgr")

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Settings</h2>

      <Card>
        <CardHeader><CardTitle>General</CardTitle></CardHeader>
        <CardContent className="space-y-3">
          <div>
            <label className="mb-1 block text-sm text-muted-foreground">Data Directory</label>
            <div className="flex items-center gap-2">
              <code className="rounded-md bg-muted px-3 py-1.5 text-sm">{basePath}</code>
            </div>
            <p className="mt-1 text-xs text-muted-foreground">All skills, configs, and database are stored here</p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader><CardTitle>About</CardTitle></CardHeader>
        <CardContent className="space-y-2 text-sm text-muted-foreground">
          <p><strong className="text-foreground">skills-mgr</strong> v0.1.0</p>
          <p>Cross-agent skill management tool</p>
          <p>Manages composable skill profiles across AI coding agents</p>
        </CardContent>
      </Card>
    </div>
  )
}
