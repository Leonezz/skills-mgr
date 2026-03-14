import { useState, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { useTheme } from "@/lib/theme"
import { getSettings, saveSettings } from "@/lib/api"
import type { SettingsPayload } from "@/lib/api"
import { check } from "@tauri-apps/plugin-updater"
import { relaunch } from "@tauri-apps/plugin-process"
import { toast } from "sonner"
import { Sun, Moon, Monitor, Loader2, Download, CheckCircle } from "lucide-react"

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
        scan_auto_on_startup: false,
      })
    })
  }, [])

  // Updater state
  const [updateStatus, setUpdateStatus] = useState<
    "idle" | "checking" | "available" | "downloading" | "ready" | "up-to-date" | "error"
  >("idle")
  const [updateVersion, setUpdateVersion] = useState("")
  const [updateError, setUpdateError] = useState("")
  const [downloadProgress, setDownloadProgress] = useState(0)

  async function checkForUpdates() {
    setUpdateStatus("checking")
    setUpdateError("")
    try {
      const update = await check()
      if (update) {
        setUpdateVersion(update.version)
        setUpdateStatus("available")
      } else {
        setUpdateStatus("up-to-date")
      }
    } catch (e) {
      setUpdateError(String(e))
      setUpdateStatus("error")
    }
  }

  async function downloadAndInstall() {
    setUpdateStatus("downloading")
    setDownloadProgress(0)
    try {
      const update = await check()
      if (!update) {
        setUpdateStatus("up-to-date")
        return
      }
      let totalLength = 0
      let downloaded = 0
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalLength = event.data.contentLength ?? 0
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength
          if (totalLength > 0) {
            setDownloadProgress(Math.round((downloaded / totalLength) * 100))
          }
        } else if (event.event === "Finished") {
          setUpdateStatus("ready")
        }
      })
      setUpdateStatus("ready")
    } catch (e) {
      setUpdateError(String(e))
      setUpdateStatus("error")
    }
  }

  async function handleRelaunch() {
    await relaunch()
  }

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
    <div className="flex flex-col flex-1 min-h-0">
      {/* Header — fixed */}
      <div className="shrink-0 pb-6">
        <h2 className="text-2xl font-bold tracking-tight">Settings</h2>
        <p className="text-sm text-muted-foreground">
          Configure skills-mgr preferences
        </p>
      </div>

      {/* Body — scrollable */}
      <div className="flex-1 min-h-0 overflow-y-auto space-y-6">

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

      {/* Skill Discovery */}
      <section className="space-y-4 rounded-xl border border-border bg-card p-6">
        <h3 className="text-base font-semibold">Skill Discovery</h3>
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <span className="text-sm font-medium">Auto-scan on Startup</span>
            <p className="text-xs text-muted-foreground">
              Automatically scan agent paths for unmanaged skills when the app starts
            </p>
          </div>
          <Switch
            checked={settings?.scan_auto_on_startup ?? false}
            onCheckedChange={(checked) => update({ scan_auto_on_startup: checked })}
            disabled={!settings || saving}
          />
        </div>
      </section>

      {/* About & Updates */}
      <section className="space-y-4 rounded-xl border border-border bg-card p-6">
        <h3 className="text-base font-semibold">About</h3>
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <span className="text-sm font-medium">Skills Manager v0.1.0</span>
            <p className="text-xs text-muted-foreground">
              Cross-agent skill management using the Agent Skills open standard
            </p>
          </div>
        </div>

        <hr className="border-border" />

        <div className="flex items-center justify-between gap-4">
          <div className="space-y-0.5 flex-1 min-w-0">
            <span className="text-sm font-medium">Updates</span>
            <p className="text-xs text-muted-foreground">
              {updateStatus === "idle" && "Check for new versions"}
              {updateStatus === "checking" && "Checking for updates..."}
              {updateStatus === "up-to-date" && "You're on the latest version"}
              {updateStatus === "available" && `Version ${updateVersion} is available`}
              {updateStatus === "downloading" && `Downloading update... ${downloadProgress}%`}
              {updateStatus === "ready" && "Update downloaded — restart to apply"}
              {updateStatus === "error" && (
                <span className="text-destructive">{updateError}</span>
              )}
            </p>
          </div>

          {updateStatus === "idle" && (
            <Button variant="outline" size="sm" onClick={checkForUpdates}>
              Check for Updates
            </Button>
          )}
          {updateStatus === "checking" && (
            <Button variant="outline" size="sm" disabled>
              <Loader2 className="h-4 w-4 animate-spin" />
              Checking...
            </Button>
          )}
          {updateStatus === "up-to-date" && (
            <Button variant="outline" size="sm" onClick={checkForUpdates}>
              <CheckCircle className="h-4 w-4 text-emerald-500" />
              Up to Date
            </Button>
          )}
          {updateStatus === "available" && (
            <Button size="sm" onClick={downloadAndInstall}>
              <Download className="h-4 w-4" />
              Download & Install
            </Button>
          )}
          {updateStatus === "downloading" && (
            <Button size="sm" disabled>
              <Loader2 className="h-4 w-4 animate-spin" />
              {downloadProgress}%
            </Button>
          )}
          {updateStatus === "ready" && (
            <Button size="sm" onClick={handleRelaunch}>
              Restart Now
            </Button>
          )}
          {updateStatus === "error" && (
            <Button variant="outline" size="sm" onClick={checkForUpdates}>
              Retry
            </Button>
          )}
        </div>
      </section>
      </div>
    </div>
  )
}
