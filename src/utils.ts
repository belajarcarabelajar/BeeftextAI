// ─── Shared Utilities ────────────────────────────────────────────────────────────

/** Estimate token count using word-based approximation (same method as Rust backend) */
export function estimateTokenCount(text: string): number {
  if (!text.trim()) return 0;
  const words = text.trim().split(/\s+/).length;
  // Word-based: words * 1.5 ≈ tokens
  const wordBased = Math.round((words * 3) / 2);
  // Char-based: chars / 4
  const charBased = Math.ceil(text.length / 4);
  return Math.max(wordBased, charBased);
}

/** Truncate text to approximately maxTokens */
export function truncateToTokens(text: string, maxTokens: number): string {
  if (maxTokens <= 0) return "";
  if (estimateTokenCount(text) <= maxTokens) return text;
  // Take ~maxTokens * 4 chars, then cut to word boundary
  let result = text.slice(0, maxTokens * 4);
  const lastSpace = result.lastIndexOf(' ');
  if (lastSpace > 0) result = result.slice(0, lastSpace);
  return result;
}

/** Format bytes to human-readable string */
export function formatBytes(bytes: number): string {
  return bytes < 1024 ? bytes + " B" : (bytes / 1024).toFixed(1) + " KB";
}
