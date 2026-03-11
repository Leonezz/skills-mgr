import { useState, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { useTheme } from "@/lib/theme"
import { getSettings, saveSettings } from "@/lib/api"
import type { SettingsPayload } from "@/lib/api"
import { toast } from "sonner"
import { Sun, Moon, Monitor } from "lucide-react"

const themeOptions = [
  { value: "light" as const, icon: Sun, label: "Light" },
  { value: "dark" as const, icon: Moon, label: "Dark" },
  { value: "system" as const, icon: Monitor, label: "System" },
]

export function Settings() {
  const { theme, setTheme } = useTheme()
  const [settings, setSettings] = useState<SettingsPayload | null>(null)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    getSettings().then(setSettings).catch(() => {
      setSettings({
        mcp_enabled: false,
        mcp_port: 3100,
        mcp_transport: "stdio",
        git_sync_enabled: false,
        git_sync_repo_url: "",
      })
    })
  }, [])

  function update(patch: Partial<SettingsPayload>) {
    if (!settings) return
    const updated = { ...settings, ...patch }
    setSettings(updated)
    setSaving(true)
    saveSettings(updated)
      .then(() => toast.success("Settings saved"))
      .catch((err) => toast.error(String(err)))
      .finally(() => setSaving(false))
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-2xl font-bold tracking-tight">Settings</h2>
        <p className="text-sm text-muted-foreground">
          Configure skills-mgr preferences
        </p>
      </div>

      {/* Appearance */}
      <section className="space-y-4 rounded-xl border border-border bg-card p-6">
        <h3 className="text-base font-semibold">Appearance</h3>
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <span className="text-sm font-medium">Theme</span>
            <p className="text-xs text-muted-foreground">
              Choose your preferred color scheme
            </p>
          </div>
          <div className="flex gap-2">
            {themeOptions.map((opt) => {
              const Icon = opt.icon
              return (
                <Button
                  key={opt.value}
                  variant={theme === opt.value ? "default" : "outline"}
                  size="sm"
                  onClick={() => setTheme(opt.value)}
                >
                  <Icon className="h-4 w-4" />
                  {opt.label}
                </Button>
              )
            })}
          </div>
        </div>
      </section>

      {/* General */}
      <section className="space-y-4 rounded-xl border border-border bg-card p-6">
        <h3 className="text-base font-semibold">General</h3>
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <span className="text-sm font-medium">Base Directory</span>
            <p className="text-xs text-muted-foreground">
              Where skills-mgr stores its data
            </p>
          </div>
          <div className="rounded-lg border border-border bg-muted px-3 py-1.5 text-sm">
            ~/.skills-mgr
          </div>
        </div>
      </section>

      {/* MCP Server */}
      <section className="space-y-4 rounded-xl border border-border bg-card p-6">
        <h3 className="text-base font-semibold">MCP Server</h3>
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <span className="text-sm font-medium">Enable MCP Server</span>
            <p className="text-xs text-muted-foreground">
              Expose skills-mgr via MCP protocol
            </p>
          </div>
          <Switch
            checked={settings?.mcp_enabled ?? false}
            onCheckedChange={(checked) => update({ mcp_enabled: checked })}
            disabled={!settings || saving}
          />
        </div>
        <div className="flex gap-4">
          <div className="flex-1 space-y-1">
            <Label className="text-xs text-muted-foreground">Port</Label>
            <Input
              value={settings?.mcp_port ?? 3100}
              onChange={(e) => {
                const port = parseInt(e.target.value, 10)
                if (!isNaN(port)) setSettings((s) => s ? { ...s, mcp_port: port } : s)
              }}
              onBlur={() => { if (settings) update({ mcp_port: settings.mcp_port }) }}
              className="h-9"
              type="number"
              disabled={!settings || saving}
            />
          </div>
          <div className="flex-1 space-y-1">
            <Label className="text-xs text-muted-foreground">Transport</Label>
            <Input
              value={settings?.mcp_transport ?? "stdio"}
              onChange={(e) => setSettings((s) => s ? { ...s, mcp_transport: e.target.value } : s)}
              onBlur={() => { if (settings) update({ mcp_transport: settings.mcp_transport }) }}
              className="h-9"
              disabled={!settings || saving}
            />
          </div>
        </div>
      </section>

      {/* Git Sync */}
      <section className="space-y-4 rounded-xl border border-border bg-card p-6">
        <h3 className="text-base font-semibold">Git Sync</h3>
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <span className="text-sm font-medium">Auto-sync with Git</span>
            <p className="text-xs text-muted-foreground">
              Automatically push/pull skill definitions from remote
            </p>
          </div>
          <Switch
            checked={settings?.git_sync_enabled ?? false}
            onCheckedChange={(checked) => update({ git_sync_enabled: checked })}
            disabled={!settings || saving}
          />
        </div>
        <div className="space-y-1">
          <Label className="text-xs text-muted-foreground">Repository URL</Label>
          <Input
            value={settings?.git_sync_repo_url ?? ""}
            onChange={(e) => setSettings((s) => s ? { ...s, git_sync_repo_url: e.target.value } : s)}
            onBlur={() => { if (settings) update({ git_sync_repo_url: settings.git_sync_repo_url }) }}
            placeholder="https://github.com/user/skills-repo.git"
            className="h-9"
            disabled={!settings || saving}
          />
        </div>
      </section>

      {/* About */}
      <section className="space-y-3 rounded-xl border border-border bg-card p-6">
        <h3 className="text-base font-semibold">About</h3>
        <p className="text-sm text-muted-foreground">Skills Manager v0.1.0</p>
        <p className="text-xs text-muted-foreground/70">
          Cross-agent skill management using the Agent Skills open standard
        </p>
      </section>
    </div>
  )
}
