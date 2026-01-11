import { useState } from 'react';
import { commands } from '@/bindings';
import { useFailableQuery } from '@/hooks/useRpcQuery';
import { createFileRoute } from '@tanstack/react-router'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { ErrorDisplay } from '@/components/error';
import { CommitDiffSection } from '@/components/CommitDiffSection';
import { cn } from '@/lib/utils';

export const Route = createFileRoute('/pulls/$nodeId/$number')({
  component: RouteComponent,
})

function RouteComponent() {
  const { number, nodeId } = Route.useParams();
  const [selectedCommitSha, setSelectedCommitSha] = useState<string | null>(null);

  const { data, error } = useFailableQuery({
    queryKey: ['pull', nodeId, number],
    queryFn: () => commands.getPull(nodeId, Number(number))
  })

  return (
    <main className="min-h-screen w-full p-4">
      <Card className="w-full h-full">
        <CardHeader>
          <CardTitle>
            {data ? data.title : `Pull Request #${number}`}
          </CardTitle>
          {data && (
            <CardDescription>
              {data.baseBranch} ← {data.headBranch}
            </CardDescription>
          )}
        </CardHeader>
        <CardContent>
          {/* Loading State */}
          {!data && !error && (
            <p className="text-muted-foreground">Loading pull request...</p>
          )}

          {/* Error State */}
          {error && <ErrorDisplay error={error} />}

          {/* Success State */}
          {data && (
            <div className="space-y-6">
              {/* PR Body Section */}
              <div className="rounded-lg border bg-muted/30 p-4">
                <h3 className="text-sm font-medium text-muted-foreground mb-2">
                  Description
                </h3>
                {data.body ? (
                  <p className="whitespace-pre-wrap text-sm">
                    {data.body}
                  </p>
                ) : (
                  <p className="text-sm text-muted-foreground italic">
                    No description provided
                  </p>
                )}
              </div>

              {/* Commits Section */}
              <div className="space-y-2">
                <h3 className="text-sm font-medium text-muted-foreground">
                  Commits ({data.commits.length})
                </h3>

                {data.commits.length === 0 ? (
                  <Alert>
                    <AlertTitle>No Commits</AlertTitle>
                    <AlertDescription>
                      No commits found in this pull request.
                    </AlertDescription>
                  </Alert>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Message</TableHead>
                        <TableHead className="hidden sm:table-cell w-[100px]">
                          Change ID
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {data.commits.map((commit) => (
                        <TableRow
                          key={commit.sha}
                          onClick={() => setSelectedCommitSha(commit.sha)}
                          className={cn(
                            "cursor-pointer hover:bg-muted/50 transition-colors",
                            selectedCommitSha === commit.sha && "bg-muted"
                          )}
                        >
                          <TableCell>
                            <div className="flex items-center gap-2">
                              <span>{commit.summary}</span>
                              {commit.description && (
                                <Tooltip>
                                  <TooltipTrigger asChild>
                                    <Button variant="ghost" size="sm" className="h-6 w-6 p-0 text-muted-foreground">
                                      ...
                                    </Button>
                                  </TooltipTrigger>
                                  <TooltipContent className="max-w-md max-h-96 overflow-auto">
                                    <p className="whitespace-pre-wrap text-sm">
                                      {commit.description}
                                    </p>
                                  </TooltipContent>
                                </Tooltip>
                              )}
                            </div>
                          </TableCell>
                          <TableCell className="hidden sm:table-cell font-mono text-xs text-muted-foreground">
                            {commit.changeId || '-'}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                )}
              </div>

              {/* Diff Section */}
              {selectedCommitSha && (
                <CommitDiffSection
                  nodeId={nodeId}
                  prNumber={Number(number)}
                  commitSha={selectedCommitSha}
                />
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </main>
  )
}
