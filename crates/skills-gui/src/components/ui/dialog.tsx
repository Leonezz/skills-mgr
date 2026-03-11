import * as React from "react"
import { cn } from "@/lib/utils"

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

interface DialogContextValue {
  open: boolean
  onOpenChange: (open: boolean) => void
}

const DialogContext = React.createContext<DialogContextValue | null>(null)

function useDialogContext(): DialogContextValue {
  const ctx = React.useContext(DialogContext)
  if (!ctx) {
    throw new Error("Dialog compound components must be used inside <Dialog>")
  }
  return ctx
}

// ---------------------------------------------------------------------------
// Dialog (root provider)
// ---------------------------------------------------------------------------

interface DialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  children: React.ReactNode
}

function Dialog({ open, onOpenChange, children }: DialogProps) {
  return (
    <DialogContext.Provider value={{ open, onOpenChange }}>
      {children}
    </DialogContext.Provider>
  )
}

// ---------------------------------------------------------------------------
// DialogOverlay
// ---------------------------------------------------------------------------

const DialogOverlay = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, onClick, ...props }, ref) => {
  const { onOpenChange } = useDialogContext()

  function handleClick(e: React.MouseEvent<HTMLDivElement>) {
    onOpenChange(false)
    onClick?.(e)
  }

  return (
    <div
      ref={ref}
      aria-hidden="true"
      className={cn(
        "fixed inset-0 z-50 bg-black/50 animate-overlay-in h-full",
        className
      )}
      onClick={handleClick}
      {...props}
    />
  )
})
DialogOverlay.displayName = "DialogOverlay"

// ---------------------------------------------------------------------------
// DialogContent
// ---------------------------------------------------------------------------

const DialogContent = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, children, ...props }, ref) => {
  const { open, onOpenChange } = useDialogContext()

  // Close on Escape
  React.useEffect(() => {
    if (!open) return
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onOpenChange(false)
    }
    document.addEventListener("keydown", handleKeyDown)
    return () => document.removeEventListener("keydown", handleKeyDown)
  }, [open, onOpenChange])

  if (!open) return null

  return (
    <>
      <DialogOverlay />
      <div
        className="fixed inset-0 z-50 flex items-center justify-center"
        onClick={() => onOpenChange(false)}
      >
        <div
          ref={ref}
          role="dialog"
          aria-modal="true"
          className={cn(
            "w-full max-w-md",
            "rounded-xl border border-border bg-card text-card-foreground shadow-xl",
            "animate-dialog-in",
            "p-6",
            className
          )}
          onClick={(e) => e.stopPropagation()}
          {...props}
        >
          {children}
        </div>
      </div>
    </>
  )
})
DialogContent.displayName = "DialogContent"

// ---------------------------------------------------------------------------
// DialogHeader
// ---------------------------------------------------------------------------

const DialogHeader = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn("mb-4 flex flex-col space-y-1.5", className)}
    {...props}
  />
))
DialogHeader.displayName = "DialogHeader"

// ---------------------------------------------------------------------------
// DialogTitle
// ---------------------------------------------------------------------------

const DialogTitle = React.forwardRef<
  HTMLHeadingElement,
  React.HTMLAttributes<HTMLHeadingElement>
>(({ className, ...props }, ref) => (
  <h2
    ref={ref}
    className={cn("text-lg font-semibold leading-none tracking-tight", className)}
    {...props}
  />
))
DialogTitle.displayName = "DialogTitle"

// ---------------------------------------------------------------------------
// DialogFooter
// ---------------------------------------------------------------------------

const DialogFooter = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn("mt-6 flex justify-end gap-2", className)}
    {...props}
  />
))
DialogFooter.displayName = "DialogFooter"

export {
  Dialog,
  DialogOverlay,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
}
