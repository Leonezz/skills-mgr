import { useEffect, useMemo, useState } from "react"
import { ArrowLeft, Layers } from "lucide-react"
import { useNavigate } from "react-router"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetFooter,
  SheetHeader,
} from "@/components/ui/sheet"
import { duplicateProfile, editProfile } from "@/lib/api"
import type { Profile, Skill } from "@/lib/schemas"
import { ProfileCompositionBlock } from "./ProfileCompositionBlock"
import { ProfileEditForm, type ProfileDraft } from "./ProfileEditForm"
import { ProfileIncludesList } from "./ProfileIncludesList"
import { ProfileSkillsList } from "./ProfileSkillsList"
import { ProfileUsedBySection } from "./ProfileUsedBySection"
import { useProfileResolution } from "./useProfileResolution"

type Mode = "view" | "edit"

interface Props {
  /** The profile to open the sheet for; null to close. */
  profile: Profile | null
  allProfiles: Profile[]
  skills: Skill[] | undefined
  /** Open the sheet directly in edit mode (e.g. from the card's ⋮ icon). */
  initialMode?: Mode
  onClose: () => void
  onDelete: (profile: Profile) => void
  /** Called when a profile is duplicated — parent should switch the sheet to the new name. */
  onDuplicated?: (newName: string) => void
}

const BASE_PROFILE_NAMES = new Set(["base", "base-layer"])

export function ProfileDetailSheet({
  profile,
  allProfiles,
  skills,
  initialMode = "view",
  onClose,
  onDelete,
  onDuplicated,
}: Props) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()

  // Navigation stack: each entry is a profile name. Only meaningful in view mode.
  const [stack, setStack] = useState<string[]>([])
  const [mode, setMode] = useState<Mode>(initialMode)
  const [draft, setDraft] = useState<ProfileDraft | null>(null)

  // Duplicate dialog state
  const [duplicateDialogOpen, setDuplicateDialogOpen] = useState(false)
  const [duplicateName, setDuplicateName] = useState("")

  // Reset stack/mode/draft every time the sheet opens for a new profile
  useEffect(() => {
    if (profile) {
      setStack([profile.name])
      setMode(initialMode)
      setDraft(
        initialMode === "edit"
          ? {
              description: profile.description ?? "",
              skills: [...profile.skills],
              includes: [...profile.includes],
            }
          : null,
      )
    } else {
      setStack([])
      setMode("view")
      setDraft(null)
    }
  }, [profile, initialMode])

  const profilesByName = useMemo(
    () => new Map(allProfiles.map((p) => [p.name, p])),
    [allProfiles],
  )
  const currentName = stack[stack.length - 1]
  const current = currentName ? profilesByName.get(currentName) ?? null : null

  // In edit mode, resolution runs against a synthetic profile built from the
  // draft so composition tiles update live. In view mode, it runs against the
  // saved profile.
  const resolutionProfile = useMemo(() => {
    if (mode === "edit" && draft && current) {
      return {
        name: current.name,
        skills: draft.skills,
        includes: draft.includes,
      }
    }
    return current
  }, [mode, draft, current])

  const resolution = useProfileResolution({
    profile: resolutionProfile,
    allProfiles,
    skills,
  })

  const saveMutation = useMutation({
    mutationFn: async () => {
      if (!current || !draft) throw new Error("No profile to save")
      const origSkills = new Set(current.skills)
      const origIncludes = new Set(current.includes)
      const draftSkills = new Set(draft.skills)
      const draftIncludes = new Set(draft.includes)

      const addSkills = draft.skills.filter((s) => !origSkills.has(s))
      const removeSkills = current.skills.filter((s) => !draftSkills.has(s))
      const addIncludes = draft.includes.filter((i) => !origIncludes.has(i))
      const removeIncludes = current.includes.filter((i) => !draftIncludes.has(i))

      const normalizedDesc = draft.description.trim()
      const origDesc = current.description ?? ""
      const description =
        normalizedDesc !== origDesc ? normalizedDesc : undefined

      return editProfile(
        current.name,
        addSkills,
        removeSkills,
        addIncludes,
        removeIncludes,
        description,
      )
    },
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      setMode("view")
      setDraft(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  const duplicateMutation = useMutation({
    mutationFn: async () => {
      if (!current) throw new Error("No profile to duplicate")
      const trimmed = duplicateName.trim()
      if (!trimmed) throw new Error("New profile name is required")
      return duplicateProfile(current.name, trimmed)
    },
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      const newName = duplicateName.trim()
      setDuplicateDialogOpen(false)
      setDuplicateName("")
      if (onDuplicated) onDuplicated(newName)
    },
    onError: (err) => toast.error(String(err)),
  })

  function openDuplicateDialog() {
    if (!current) return
    setDuplicateName(`${current.name}-copy`)
    setDuplicateDialogOpen(true)
  }

  function closeDuplicateDialog() {
    setDuplicateDialogOpen(false)
    setDuplicateName("")
  }

  function startEdit() {
    if (!current) return
    setDraft({
      description: current.description ?? "",
      skills: [...current.skills],
      includes: [...current.includes],
    })
    setMode("edit")
  }

  function cancelEdit() {
    setMode("view")
    setDraft(null)
  }

  function handleNavigate(name: string) {
    if (mode === "edit") return // disabled in edit mode
    if (!profilesByName.has(name)) return
    setStack((prev) => [...prev, name])
  }

  function handleBack() {
    if (mode === "edit") return
    setStack((prev) => (prev.length > 1 ? prev.slice(0, -1) : prev))
  }

  function handleClose() {
    // In edit mode we simply discard. Could confirm here if needed.
    cancelEdit()
    onClose()
  }

  function handleSkillJump(name: string) {
    // Cross-page jump: the Skills page reads ?detail=<name> on mount
    // and auto-opens that skill's detail sheet.
    handleClose()
    navigate(`/skills?detail=${encodeURIComponent(name)}`)
  }

  const open = profile !== null
  const isBase = current ? BASE_PROFILE_NAMES.has(current.name) : false
  const activeCount = current?.active_projects.length ?? 0
  const canGoBack = mode === "view" && stack.length > 1

  const includeSuggestions = useMemo(() => {
    if (!current || !draft) return []
    return allProfiles
      .map((p) => p.name)
      .filter((n) => n !== current.name && !draft.includes.includes(n))
  }, [allProfiles, current, draft])

  const skillSuggestions = useMemo(
    () => skills?.map((s) => s.name) ?? [],
    [skills],
  )

  const hasChanges = useMemo(() => {
    if (!current || !draft) return false
    if ((draft.description ?? "").trim() !== (current.description ?? "")) {
      return true
    }
    const origSkills = new Set(current.skills)
    const origIncludes = new Set(current.includes)
    if (draft.skills.length !== current.skills.length) return true
    if (draft.includes.length !== current.includes.length) return true
    if (!draft.skills.every((s) => origSkills.has(s))) return true
    if (!draft.includes.every((i) => origIncludes.has(i))) return true
    return false
  }, [current, draft])

  return (
    <>
    <Sheet open={open} onOpenChange={(o) => { if (!o) handleClose() }}>
      <SheetContent>
        <SheetHeader onClose={handleClose}>
          <div className="flex items-center gap-3">
            {canGoBack && (
              <button
                type="button"
                onClick={handleBack}
                className="rounded p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                title="Back"
              >
                <ArrowLeft className="h-4 w-4" />
              </button>
            )}
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[8px] bg-primary/10">
              <Layers className="h-4 w-4 text-primary" />
            </div>
            <div className="min-w-0">
              <h3 className="truncate text-lg font-semibold">
                {current?.name ?? "—"}
              </h3>
              {mode === "edit" ? (
                <p className="truncate text-[11px] uppercase tracking-wide text-primary">
                  Editing
                </p>
              ) : (
                canGoBack && (
                  <p className="truncate text-[11px] text-muted-foreground">
                    from {stack[0]}
                  </p>
                )
              )}
            </div>
          </div>
        </SheetHeader>

        <SheetBody className="space-y-5">
          {current && (
            <>
              {/* Badges */}
              {(isBase || activeCount > 0) && (
                <div className="flex flex-wrap gap-2">
                  {isBase && <Badge variant="secondary">BASE</Badge>}
                  {activeCount > 0 && (
                    <Badge variant="default">
                      Active in {activeCount} project
                      {activeCount !== 1 ? "s" : ""}
                    </Badge>
                  )}
                </div>
              )}

              {mode === "edit" && draft ? (
                <>
                  <ProfileEditForm
                    profileName={current.name}
                    draft={draft}
                    onChange={setDraft}
                    skillSuggestions={skillSuggestions}
                    includeSuggestions={includeSuggestions}
                  />

                  <hr className="border-border" />

                  {/* Live composition preview as user edits */}
                  <ProfileCompositionBlock resolution={resolution} />

                  <hr className="border-border" />

                  <ProfileUsedBySection
                    profile={current}
                    allProfiles={allProfiles}
                    onNavigateProfile={() => {
                      /* disabled in edit mode */
                    }}
                  />
                </>
              ) : (
                <>
                  {/* Description */}
                  <div className="space-y-2">
                    <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                      Description
                    </p>
                    <p className="text-sm leading-relaxed">
                      {current.description || (
                        <span className="text-muted-foreground">
                          No description
                        </span>
                      )}
                    </p>
                  </div>

                  <hr className="border-border" />

                  <ProfileCompositionBlock resolution={resolution} />

                  {current.includes.length > 0 && (
                    <>
                      <hr className="border-border" />
                      <ProfileIncludesList
                        includes={current.includes}
                        allProfiles={allProfiles}
                        skills={skills}
                        onNavigate={handleNavigate}
                      />
                    </>
                  )}

                  <hr className="border-border" />

                  <ProfileSkillsList
                    directSkills={resolution.directSkills}
                    inheritedSkills={resolution.inheritedSkills}
                    inheritedFrom={resolution.inheritedFrom}
                    skills={skills}
                    onSkillClick={handleSkillJump}
                  />

                  <hr className="border-border" />

                  <ProfileUsedBySection
                    profile={current}
                    allProfiles={allProfiles}
                    onNavigateProfile={handleNavigate}
                  />
                </>
              )}
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
              <Button
                className="flex-1"
                onClick={startEdit}
                disabled={!current}
              >
                Edit Profile
              </Button>
              <Button
                variant="outline"
                className="shrink-0"
                onClick={openDuplicateDialog}
                disabled={!current}
              >
                Duplicate
              </Button>
              {current && !isBase && (
                <Button
                  variant="outline"
                  className="shrink-0 border-destructive/40 text-destructive hover:bg-destructive/10"
                  onClick={() => onDelete(current)}
                >
                  Delete
                </Button>
              )}
            </>
          )}
        </SheetFooter>
      </SheetContent>
    </Sheet>

      {/* Duplicate Profile Dialog */}
      <Dialog
        open={duplicateDialogOpen}
        onOpenChange={(o) => { if (!o) closeDuplicateDialog() }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Duplicate Profile</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Create a copy of{" "}
              <span className="font-medium text-foreground">
                {current?.name}
              </span>{" "}
              with the same skills and includes. The copy starts fresh — it
              won&apos;t be linked to any projects.
            </p>
            <div className="space-y-2">
              <Label>New Profile Name</Label>
              <Input
                value={duplicateName}
                onChange={(e) => setDuplicateName(e.target.value)}
                placeholder="e.g. typescript-dev-copy"
                autoFocus
                onKeyDown={(e) => {
                  if (
                    e.key === "Enter" &&
                    duplicateName.trim() &&
                    !duplicateMutation.isPending
                  ) {
                    duplicateMutation.mutate()
                  }
                }}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={closeDuplicateDialog}
              disabled={duplicateMutation.isPending}
            >
              Cancel
            </Button>
            <Button
              onClick={() => duplicateMutation.mutate()}
              disabled={!duplicateName.trim() || duplicateMutation.isPending}
            >
              {duplicateMutation.isPending ? "Duplicating..." : "Duplicate"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
