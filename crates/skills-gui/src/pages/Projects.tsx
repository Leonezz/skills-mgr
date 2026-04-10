import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import {
  listProjects,
  listProfiles,
  addProject,
  removeProject,
  activateProject,
  deactivateProject,
} from "@/lib/api"
import { open } from "@tauri-apps/plugin-dialog"
import { toast } from "sonner"
import {
  Plus,
  FolderOpen,
  FolderKanban,
  Pencil,
  Play,
  Square,
} from "lucide-react"
import type { Project } from "@/lib/schemas"
import { ProjectDetailSheet } from "./projects/ProjectDetailSheet"

export function Projects() {
  const queryClient = useQueryClient()
  const { data: projects, isLoading } = useQuery({
    queryKey: ["projects"],
    queryFn: listProjects,
    refetchInterval: 5_000,
  })
  const { data: profilesData } = useQuery({
    queryKey: ["profiles"],
    queryFn: listProfiles,
  })

  const [showAdd, setShowAdd] = useState(false)
  const [addPath, setAddPath] = useState("")
  const [showRemove, setShowRemove] = useState<Project | null>(null)
  const [detailPath, setDetailPath] = useState<string | null>(null)
  const [detailMode, setDetailMode] = useState<"view" | "edit">("view")

  const profileNames = profilesData?.profiles.map((p) => p.name) ?? []

  const addMutation = useMutation({
    mutationFn: () => addProject(addPath),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["projects"] })
      closeAdd()
    },
    onError: (err) => toast.error(String(err)),
  })

  const removeMutation = useMutation({
    mutationFn: (path: string) => removeProject(path),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["projects"] })
      setShowRemove(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  function closeAdd() {
    setShowAdd(false)
    setAddPath("")
  }

  function openDetail(project: Project, mode: "view" | "edit" = "view") {
    setDetailMode(mode)
    setDetailPath(project.path)
  }

  async function handleActivate(project: Project) {
    try {
      const msg = await activateProject(project.path)
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["projects", "profiles", "logs"] })
    } catch (err) {
      toast.error(String(err))
    }
  }

  async function handleDeactivate(project: Project) {
    try {
      const msg = await deactivateProject(project.path)
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["projects", "profiles", "logs"] })
    } catch (err) {
      toast.error(String(err))
    }
  }

  async function handleBrowseAdd() {
    const selected = await open({ directory: true, title: "Select project folder" })
    if (selected) setAddPath(selected as string)
  }

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* Header — fixed */}
      <div className="shrink-0 flex items-center justify-between pb-6">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Projects</h2>
          <p className="text-sm text-muted-foreground">
            Manage skill deployments per project
          </p>
        </div>
        <Button onClick={() => setShowAdd(true)}>
          <Plus className="h-4 w-4" />
          Add Project
        </Button>
      </div>

      {/* Add Project Dialog */}
      <Dialog open={showAdd} onOpenChange={(o) => { if (!o) closeAdd() }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Project</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Project Directory</Label>
              <p className="text-xs text-muted-foreground">
                Select the root directory of your project
              </p>
              <div className="flex gap-2">
                <Input
                  value={addPath}
                  onChange={(e) => setAddPath(e.target.value)}
                  placeholder="/path/to/your/project"
                  className="flex-1"
                />
                <Button type="button" variant="outline" size="sm" onClick={handleBrowseAdd}>
                  <FolderOpen className="h-4 w-4" />
                </Button>
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeAdd}>
              Cancel
            </Button>
            <Button
              onClick={() => addMutation.mutate()}
              disabled={!addPath || addMutation.isPending}
            >
              {addMutation.isPending ? "Adding..." : "Add Project"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Project Detail Sheet */}
      <ProjectDetailSheet
        project={detailPath ? projects?.find((p) => p.path === detailPath) ?? null : null}
        profileNames={profileNames}
        initialMode={detailMode}
        onClose={() => setDetailPath(null)}
        onRemove={(project) => {
          setDetailPath(null)
          setShowRemove(project)
        }}
      />

      {/* Remove Project Confirmation */}
      <Dialog open={showRemove !== null} onOpenChange={(o) => { if (!o) setShowRemove(null) }}>
        <DialogContent className="max-w-[420px] space-y-4 p-7">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-lg font-bold text-destructive">
              !
            </div>
            <h2 className="text-lg font-semibold">Remove Project?</h2>
          </div>
          <p className="text-sm leading-relaxed text-muted-foreground">
            This will unregister <span className="font-medium text-foreground">{showRemove?.name}</span> from
            skills-mgr tracking. Placed skill files will not be deleted.
          </p>
          <hr className="border-border" />
          <div className="flex gap-3">
            <Button variant="outline" className="flex-1" onClick={() => setShowRemove(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              className="flex-1"
              onClick={() => { if (showRemove) removeMutation.mutate(showRemove.path) }}
              disabled={removeMutation.isPending}
            >
              {removeMutation.isPending ? "Removing..." : "Remove"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Project List — scrollable */}
      <div className="flex-1 min-h-0 overflow-y-auto">
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : projects && projects.length > 0 ? (
        <div className="space-y-3">
          {projects.map((project: Project, index: number) => {
            const isActive = project.active_profiles.length > 0
            return (
              <div
                key={project.path}
                role="button"
                tabIndex={0}
                onClick={() => openDetail(project, "view")}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault()
                    openDetail(project, "view")
                  }
                }}
                className="animate-list-item group cursor-pointer rounded-xl border border-border bg-card p-5 transition-colors hover:border-primary/30 focus:border-primary/50 focus:outline-none"
                style={{ animationDelay: `${index * 50}ms` }}
              >
                <div className="flex items-start gap-4">
                  {/* Icon */}
                  <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[10px] bg-primary/10">
                    <FolderKanban className="h-5 w-5 text-primary" />
                  </div>

                  {/* Info */}
                  <div className="flex-1 space-y-2 min-w-0">
                    <div>
                      <span className="text-[15px] font-semibold">{project.name}</span>
                      <p className="text-xs text-muted-foreground truncate">{project.path}</p>
                    </div>

                    {/* Meta row */}
                    <div className="flex items-center gap-4 text-xs text-muted-foreground">
                      <span>
                        {project.linked_profiles.length} profile{project.linked_profiles.length !== 1 ? "s" : ""} linked
                      </span>
                      {isActive && (
                        <span className="text-emerald-600 dark:text-emerald-400">
                          {project.active_profiles.length} active
                        </span>
                      )}
                      <span>
                        {project.placement_count} placement{project.placement_count !== 1 ? "s" : ""}
                      </span>
                    </div>

                    {/* Linked profiles as badges */}
                    {project.linked_profiles.length > 0 && (
                      <div className="flex flex-wrap gap-1.5">
                        {project.linked_profiles.map((profileName) => {
                          const active = project.active_profiles.includes(profileName)
                          return (
                            <Badge
                              key={profileName}
                              variant={active ? "default" : "secondary"}
                              className="text-[11px]"
                            >
                              {profileName}
                              {active && (
                                <span className="ml-1 opacity-70">active</span>
                              )}
                            </Badge>
                          )
                        })}
                      </div>
                    )}
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-1.5 shrink-0">
                    {isActive ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-8 border-orange-300 text-orange-600 hover:bg-orange-50 dark:border-orange-700 dark:text-orange-400 dark:hover:bg-orange-950"
                        onClick={(e) => {
                          e.stopPropagation()
                          handleDeactivate(project)
                        }}
                      >
                        <Square className="h-3.5 w-3.5" />
                        Deactivate
                      </Button>
                    ) : project.linked_profiles.length > 0 ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-8 border-emerald-300 text-emerald-600 hover:bg-emerald-50 dark:border-emerald-700 dark:text-emerald-400 dark:hover:bg-emerald-950"
                        onClick={(e) => {
                          e.stopPropagation()
                          handleActivate(project)
                        }}
                      >
                        <Play className="h-3.5 w-3.5" />
                        Activate
                      </Button>
                    ) : null}
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        openDetail(project, "edit")
                      }}
                      className="rounded p-1.5 text-muted-foreground transition-colors hover:text-foreground hover:bg-muted"
                      title="Edit project"
                    >
                      <Pencil className="h-4 w-4" />
                    </button>
                  </div>
                </div>
              </div>
            )
          })}
        </div>
      ) : (
        <div className="rounded-xl border border-dashed border-border p-12 text-center">
          <FolderKanban className="mx-auto h-10 w-10 text-muted-foreground/40" />
          <p className="mt-3 text-sm text-muted-foreground">
            No projects registered yet. Add a project to start activating profiles.
          </p>
          <Button variant="outline" className="mt-4" onClick={() => setShowAdd(true)}>
            <Plus className="h-4 w-4" />
            Add Project
          </Button>
        </div>
      )}
      </div>
    </div>
  )
}
