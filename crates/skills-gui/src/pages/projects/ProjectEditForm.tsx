import { Label } from "@/components/ui/label"
import { TagInput } from "@/components/ui/tag-input"

export interface ProjectDraft {
  linkedProfiles: string[]
}

interface Props {
  projectName: string
  projectPath: string
  draft: ProjectDraft
  onChange: (next: ProjectDraft) => void
  profileSuggestions: string[]
}

export function ProjectEditForm({
  projectName,
  projectPath,
  draft,
  onChange,
  profileSuggestions,
}: Props) {
  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <Label className="text-xs text-muted-foreground">Project</Label>
        <p className="text-sm font-medium">{projectName}</p>
        <p className="text-xs text-muted-foreground truncate">{projectPath}</p>
      </div>
      <hr className="border-border" />
      <div className="space-y-2">
        <Label>Linked Profiles</Label>
        <p className="text-xs text-muted-foreground">
          Attach profiles to this project. Use Activate to deploy them.
        </p>
        <TagInput
          value={draft.linkedProfiles}
          onChange={(tags) => onChange({ ...draft, linkedProfiles: tags })}
          suggestions={profileSuggestions}
          placeholder="Search profiles..."
        />
      </div>
    </div>
  )
}
