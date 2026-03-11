import * as React from "react"
import { X } from "lucide-react"
import { cn } from "@/lib/utils"

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface TagInputProps {
  value: string[]
  onChange: (tags: string[]) => void
  suggestions?: string[]
  placeholder?: string
  className?: string
}

// ---------------------------------------------------------------------------
// TagInput
// ---------------------------------------------------------------------------

function TagInput({
  value,
  onChange,
  suggestions = [],
  placeholder = "Add tag…",
  className,
}: TagInputProps) {
  const [inputValue, setInputValue] = React.useState("")
  const [isOpen, setIsOpen] = React.useState(false)
  const inputRef = React.useRef<HTMLInputElement>(null)
  const containerRef = React.useRef<HTMLDivElement>(null)

  const filteredSuggestions = React.useMemo(() => {
    const query = inputValue.trim().toLowerCase()
    return suggestions.filter(
      (s) =>
        !value.includes(s) &&
        (query === "" || s.toLowerCase().includes(query))
    )
  }, [inputValue, suggestions, value])

  function addTag(tag: string) {
    const trimmed = tag.trim()
    if (trimmed === "" || value.includes(trimmed)) return
    onChange([...value, trimmed])
    setInputValue("")
    setIsOpen(false)
    inputRef.current?.focus()
  }

  function removeTag(tag: string) {
    onChange(value.filter((t) => t !== tag))
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault()
      if (filteredSuggestions.length > 0 && inputValue.trim() === "") {
        addTag(filteredSuggestions[0])
      } else {
        addTag(inputValue)
      }
    } else if (e.key === "Backspace" && inputValue === "" && value.length > 0) {
      onChange(value.slice(0, -1))
    } else if (e.key === "Escape") {
      setIsOpen(false)
      inputRef.current?.blur()
    }
  }

  function handleInputChange(e: React.ChangeEvent<HTMLInputElement>) {
    setInputValue(e.target.value)
    setIsOpen(true)
  }

  function handleInputFocus() {
    setIsOpen(true)
  }

  // Close dropdown when clicking outside
  React.useEffect(() => {
    function handlePointerDown(e: PointerEvent) {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setIsOpen(false)
      }
    }
    document.addEventListener("pointerdown", handlePointerDown)
    return () => document.removeEventListener("pointerdown", handlePointerDown)
  }, [])

  const showDropdown = isOpen && filteredSuggestions.length > 0

  return (
    <div ref={containerRef} className={cn("relative", className)}>
      {/* Tag container + input */}
      <div
        className={cn(
          "flex min-h-9 w-full flex-wrap gap-1.5 rounded-md border border-input bg-transparent px-3 py-1.5",
          "text-sm text-foreground shadow-sm transition-colors",
          "focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2"
        )}
        onClick={() => inputRef.current?.focus()}
      >
        {value.map((tag) => (
          <span
            key={tag}
            className={cn(
              "inline-flex items-center gap-1 rounded px-2 py-0.5 text-xs font-medium",
              "bg-indigo-100 text-indigo-700 dark:bg-indigo-900/40 dark:text-indigo-300"
            )}
          >
            {tag}
            <button
              type="button"
              aria-label={`Remove ${tag}`}
              onClick={(e) => {
                e.stopPropagation()
                removeTag(tag)
              }}
              className="ml-0.5 rounded-sm opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus:ring-1 focus:ring-ring"
            >
              <X className="h-3 w-3" />
            </button>
          </span>
        ))}

        <input
          ref={inputRef}
          type="text"
          value={inputValue}
          onChange={handleInputChange}
          onFocus={handleInputFocus}
          onKeyDown={handleKeyDown}
          placeholder={value.length === 0 ? placeholder : ""}
          className={cn(
            "min-w-24 flex-1 bg-transparent outline-none placeholder:text-muted-foreground"
          )}
        />
      </div>

      {/* Suggestions dropdown */}
      {showDropdown && (
        <ul
          role="listbox"
          className={cn(
            "absolute left-0 right-0 top-full z-50 mt-1 max-h-48 overflow-y-auto",
            "rounded-md border border-border bg-popover text-popover-foreground shadow-md"
          )}
        >
          {filteredSuggestions.map((suggestion) => (
            <li key={suggestion} role="option" aria-selected={false}>
              <button
                type="button"
                className={cn(
                  "w-full px-3 py-2 text-left text-sm",
                  "hover:bg-accent hover:text-accent-foreground",
                  "focus:bg-accent focus:text-accent-foreground focus:outline-none"
                )}
                onPointerDown={(e) => {
                  // Use pointerdown to run before the input blur closes the dropdown
                  e.preventDefault()
                  addTag(suggestion)
                }}
              >
                {suggestion}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

export { TagInput }
export type { TagInputProps }
