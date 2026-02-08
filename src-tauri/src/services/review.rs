use crate::models::PatchId;
use sha1::{Digest, Sha1};

/// Compute git patch-id for a file patch
///
/// This computes a content-based hash similar to git's patch-id algorithm:
/// - Only includes addition and deletion lines (no context)
/// - Normalizes whitespace
/// - Returns SHA-1 hash as hex string
pub fn compute_file_patch_id(patch: &git2::Patch) -> Result<PatchId, git2::Error> {
    let mut hasher = Sha1::new();

    // Iterate through all hunks in the patch
    for hunk_idx in 0..patch.num_hunks() {
        let (_hunk, num_lines) = patch.hunk(hunk_idx)?;

        // Process each line in the hunk
        for line_idx in 0..num_lines {
            let line = patch.line_in_hunk(hunk_idx, line_idx)?;

            // Only include additions and deletions in the hash
            match line.origin_value() {
                git2::DiffLineType::Addition | git2::DiffLineType::Deletion => {
                    // Add the line content to the hash
                    // Normalize by trimming trailing whitespace
                    let content = line.content();
                    let normalized = String::from_utf8_lossy(content);
                    let trimmed = normalized.trim_end();

                    // Include the origin (+ or -) and the normalized content
                    hasher.update([line.origin() as u8]);
                    hasher.update(trimmed.as_bytes());
                    hasher.update(b"\n");
                }
                _ => {
                    // Skip context lines and other line types
                }
            }
        }
    }

    // Compute final hash and return as hex string
    let result = hasher.finalize();
    Ok(PatchId::from(format!("{:x}", result)))
}
