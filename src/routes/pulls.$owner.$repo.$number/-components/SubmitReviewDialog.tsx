import { useState } from "react"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Textarea } from "@/components/ui/textarea"

import { type ReviewEvent, useSubmitReview } from "../-hooks/useSubmitReview"

const reviewOptions = [
  {
    event: "APPROVE" as const,
    label: "Approve",
    buttonVariant: "default" as const,
  },
  {
    event: "COMMENT" as const,
    label: "Comment",
    buttonVariant: "outline" as const,
  },
  {
    event: "REQUEST_CHANGES" as const,
    label: "Request Changes",
    buttonVariant: "destructive" as const,
  },
]

export function SubmitReviewDialog({
  open,
  onOpenChange,
  owner,
  repo,
  pullNumber,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  owner: string
  repo: string
  pullNumber: number
}) {
  const [event, setEvent] = useState<ReviewEvent>("APPROVE")
  const [body, setBody] = useState("")
  const submitReview = useSubmitReview()

  const selectedOption = reviewOptions.find((o) => o.event === event)!
  const bodyRequired = event === "REQUEST_CHANGES" || event === "COMMENT"
  const canSubmit = !bodyRequired || body.trim().length > 0

  const handleSubmit = () => {
    submitReview.mutate(
      { owner, repo, pullNumber, event, body: body.trim() },
      {
        onSuccess: () => {
          setBody("")
          setEvent("APPROVE")
          onOpenChange(false)
        },
      },
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Submit Review</DialogTitle>
        </DialogHeader>

        <Textarea
          placeholder="Leave a comment (optional for approve)"
          value={body}
          onChange={(e) => setBody(e.target.value)}
          className="min-h-24"
        />

        <div className="flex gap-2">
          {reviewOptions.map((option) => (
            <Button
              key={option.event}
              type="button"
              variant={event === option.event ? option.buttonVariant : "ghost"}
              onClick={() => setEvent(option.event)}
              className="flex-1"
            >
              {option.label}
            </Button>
          ))}
        </div>

        <DialogFooter>
          <Button
            variant={selectedOption.buttonVariant}
            onClick={handleSubmit}
            disabled={!canSubmit || submitReview.isPending}
          >
            {submitReview.isPending
              ? "Submitting..."
              : `Submit: ${selectedOption.label}`}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
