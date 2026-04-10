import { useEffect, useMemo, useState } from "react"
import { FolderKanban, ExternalLink, Play, Square } from "lucide-react"
import { useNavigate } from "react-router"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetFooter,
  SheetHeader,
} from "@/components/ui/sheet"
import {
  getProjectDetail,
  linkProfileToProject,
  unlinkProfileFromProject,
  activateProject,
  deactivateProject,
  revealPath,
} from "@/lib/api"
import type { Project } from "@/lib/schemas"
import { ProjectEditForm, type ProjectDraft } from "./ProjectEditForm"
import { ProjectLinkedProfilesList } from "./ProjectLinkedProfilesList"
import { ProjectPlacementsBlock } from "./ProjectPlacementsBlock"
import { ProjectRecentActivity } from "./ProjectRecentActivity"

type Mode = "view" | "edit"

interface Props {
  project: Project | null
  profileNames: string[]
  initialMode?: Mode
  onClose: () => void
  onRemove: (project: Project) => void
}

export function ProjectDetailSheet({
  project,
  profileNames,
  initialMode = "view",
  onClose,
  onRemove,
}: Props) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()

  const [mode, setMode] = useState<Mode>(initialMode)
  const [draft, setDraft] = useState<ProjectDraft | null>(null)

  // Detail query — only fetches when the sheet is open for a specific project
  const { data: detail, isLoading } = useQuery({
    queryKey: ["projectDetail", project?.path],
    queryFn: () => getProjectDetail(project!.path),
    enabled: project !== null,
  })

  useEffect(() => {
    if (project) {
      setMode(initialMode)
      setDraft(
        initialMode === "edit"
          ? { linkedProfiles: [...project.linked_profiles] }
          : null,
      )
    } else {
      setMode("view")
      setDraft(null)
    }
  }, [project, initialMode])

  // Save: diff draft linked profiles against current, call link/unlink per-change
  const saveMutation = useMutation({
    mutationFn: async () => {
      if (!project || !draft) throw new Error("No project to save")
      const current = new Set(project.linked_profiles)
      const desired = new Set(draft.linkedProfiles)
      const toLink = draft.linkedProfiles.filter((p) => !current.has(p))
      const toUnlink = project.linked_profiles.filter((p) => !desired.has(p))

      const errors: string[] = []
      for (const name of toUnlink) {
        try {
          await unlinkProfileFromProject(project.path, name)
        } catch (err) {
          errors.push(`Unlink ${name}: ${err}`)
        }
      }
      for (const name of toLink) {
        try {
          await linkProfileToProject(project.path, name)
        } catch (err) {
          errors.push(`Link ${name}: ${err}`)
        }
      }
      if (errors.length > 0) {
        throw new Error(errors.join("; "))
      }
      return "Profiles updated"
    },
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["projects"] })
      queryClient.invalidateQueries({
        queryKey: ["projectDetail", project?.path],
      })
      setMode("view")
      setDraft(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  function startEdit() {
    if (!project) return
    setDraft({ linkedProfiles: [...project.linked_profiles] })
    setMode("edit")
  }

  function cancelEdit() {
    setMode("view")
    setDraft(null)
  }

  function handleClose() {
    cancelEdit()
    onClose()
  }

  async function handleActivate() {
    if (!project) return
    try {
      const msg = await activateProject(project.path)
      toast.success(msg)
      queryClient.invalidateQueries({
        queryKey: ["projects", "profiles", "logs"],
      })
      queryClient.invalidateQueries({
        queryKey: ["projectDetail", project.path],
      })
    } catch (err) {
      toast.error(String(err))
    }
  }

  async function handleDeactivate() {
    if (!project) return
    try {
      const msg = await deactivateProject(project.path)
      toast.success(msg)
      queryClient.invalidateQueries({
        queryKey: ["projects", "profiles", "logs"],
      })
      queryClient.invalidateQueries({
        queryKey: ["projectDetail", project.path],
      })
    } catch (err) {
      toast.error(String(err))
    }
  }

  function handleProfileNavigate(profileName: string) {
    handleClose()
    navigate(`/profiles?detail=${encodeURIComponent(profileName)}`)
  }

  async function handleReveal() {
    if (!project) return
    try {
      await revealPath(project.path)
    } catch (err) {
      toast.error(String(err))
    }
  }

  const open = project !== null
  const isActive = (project?.active_profiles.length ?? 0) > 0

  const hasChanges = useMemo(() => {
    if (!project || !draft) return false
    const current = new Set(project.linked_profiles)
    if (draft.linkedProfiles.length !== current.size) return true
    return !draft.linkedProfiles.every((p) => current.has(p))
  }, [project, draft])

  const filteredSuggestions = useMemo(() => {
    if (!draft) return profileNames
    return profileNames.filter((n) => !draft.linkedProfiles.includes(n))
  }, [profileNames, draft])

  return (
    <Sheet open={open} onOpenChange={(o) => { if (!o) handleClose() }}>
      <SheetContent>
        <SheetHeader onClose={handleClose}>
          <div className="flex items-center gap-3">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[8px] bg-primary/10">
              <FolderKanban className="h-4 w-4 text-primary" />
            </div>
            <div className="min-w-0">
              <h3 className="truncate text-lg font-semibold">
                {project?.name ?? "—"}
              </h3>
              {mode === "edit" ? (
                <p className="truncate text-[11px] uppercase tracking-wide text-primary">
                  Editing
                </p>
              ) : (
                <p className="truncate text-[11px] font-mono text-muted-foreground">
                  {project?.path}
                </p>
              )}
            </div>
          </div>
        </SheetHeader>

        <SheetBody className="space-y-5">
          {project && (
            <>
              {/* Badges */}
              <div className="flex flex-wrap items-center gap-2">
                {isActive && (
                  <Badge variant="default">
                    {project.active_profiles.length} active profile
                    {project.active_profiles.length !== 1 ? "s" : ""}
                  </Badge>
                )}
                <Badge variant="secondary">
                  {project.linked_profiles.length} linked
                </Badge>
                <Badge variant="secondary">
                  {project.placement_count} placement
                  {project.placement_count !== 1 ? "s" : ""}
                </Badge>
              </div>

              {mode === "edit" && draft ? (
                <ProjectEditForm
                  projectName={project.name}
                  projectPath={project.path}
                  draft={draft}
                  onChange={setDraft}
                  profileSuggestions={filteredSuggestions}
                />
              ) : isLoading ? (
                <p className="text-sm text-muted-foreground">Loading...</p>
              ) : detail ? (
                <>
                  {/* Path + Reveal */}
                  <div className="space-y-2">
                    <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      Location
                    </p>
                    <div className="flex items-center gap-2">
                      <p className="flex-1 truncate text-xs font-mono text-muted-foreground">
                        {detail.path}
                      </p>
                      <button
                        type="button"
                        onClick={handleReveal}
                        className="shrink-0 rounded p-1 text-muted-foreground transition-colors hover:text-foreground hover:bg-muted"
                        title="Reveal in Finder"
                      >
                        <ExternalLink className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  </div>

                  <hr className="border-border" />

                  <ProjectLinkedProfilesList
                    profiles={detail.linked_profiles}
                    onNavigate={handleProfileNavigate}
                  />

                  <hr className="border-border" />

                  <ProjectPlacementsBlock
                    placementsByAgent={detail.placements_by_agent}
                  />

                  <hr className="border-border" />

                  <ProjectRecentActivity activity={detail.recent_activity} />
                </>
              ) : null}
            </>
          )}
        </SheetBody>

        <SheetFooter>
          {mode === "edit" ? (
            <>
              <Button
                variant="outline"
                className="flex-1"
                onClick={cancelEdit}
                disabled={saveMutation.isPending}
              >
                Cancel
              </Button>
              <Button
                className="flex-1"
                onClick={() => saveMutation.mutate()}
                disabled={!hasChanges || saveMutation.isPending}
              >
                {saveMutation.isPending ? "Saving..." : "Save Changes"}
              </Button>
            </>
          ) : (
            <>
              {isActive ? (
                <Button
                  variant="outline"
                  className="shrink-0 border-orange-300 text-orange-600 hover:bg-orange-50 dark:border-orange-700 dark:text-orange-400 dark:hover:bg-orange-950"
                  onClick={handleDeactivate}
                >
                  <Square className="h-3.5 w-3.5" />
                  Deactivate
                </Button>
              ) : project && project.linked_profiles.length > 0 ? (
                <Button
                  variant="outline"
                  className="shrink-0 border-emerald-300 text-emerald-600 hover:bg-emerald-50 dark:border-emerald-700 dark:text-emerald-400 dark:hover:bg-emerald-950"
                  onClick={handleActivate}
                >
                  <Play className="h-3.5 w-3.5" />
                  Activate
                </Button>
              ) : null}
              <Button className="flex-1" onClick={startEdit} disabled={!project}>
                Edit Project
              </Button>
              {project && (
                <Button
                  variant="outline"
                  className="shrink-0 border-destructive/40 text-destructive hover:bg-destructive/10"
                  onClick={() => onRemove(project)}
                >
                  Remove
                </Button>
              )}
            </>
          )}
        </SheetFooter>
      </SheetContent>
    </Sheet>
  )
}
