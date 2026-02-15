import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTrigger } from "@/components/ui/dialog"

export function OpenLocalRepoDialog() {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button className="w-20">Open</Button>
      </DialogTrigger>
      <DialogContent></DialogContent>
    </Dialog>
  )
}
