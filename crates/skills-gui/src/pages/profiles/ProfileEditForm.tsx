import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { TagInput } from "@/components/ui/tag-input"

export interface ProfileDraft {
  description: string
  skills: string[]
  includes: string[]
}

interface Props {
  /** Profile being edited — read-only fields like name come from here. */
  profileName: string
  draft: ProfileDraft
  onChange: (next: ProfileDraft) => void
  /** Suggestions for the skills tag input. */
  skillSuggestions: string[]
  /** Suggestions for the includes tag input (excluding self and existing). */
  includeSuggestions: string[]
}

export function ProfileEditForm({
  profileName,
  draft,
  onChange,
  skillSuggestions,
  includeSuggestions,
}: Props) {
  return (
    <div className="space-y-5">
      <div className="space-y-2">
        <Label>Profile Name</Label>
        <Input value={profileName} disabled />
      </div>

      <div className="space-y-2">
        <Label>Description</Label>
        <Input
          value={draft.description}
          onChange={(e) => onChange({ ...draft, description: e.target.value })}
          placeholder="Profile description..."
        />
      </div>

      <div className="space-y-2">
        <Label>Compose from Profiles</Label>
        <p className="text-xs text-muted-foreground">
          Inherit skills from existing profiles
        </p>
        <TagInput
          value={draft.includes}
          onChange={(tags) => onChange({ ...draft, includes: tags })}
          suggestions={includeSuggestions}
          placeholder="+ Add profile"
        />
      </div>

      <div className="space-y-2">
        <Label>Direct Skills ({draft.skills.length})</Label>
        <TagInput
          value={draft.skills}
          onChange={(tags) => onChange({ ...draft, skills: tags })}
          suggestions={skillSuggestions}
          placeholder="+ Add skill"
        />
      </div>
    </div>
  )
}
