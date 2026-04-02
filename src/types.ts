// ─── Shared Types ────────────────────────────────────────────────────────────────

export interface Snippet {
  uuid: string;
  name: string;
  keyword: string;
  snippet: string;
  description: string;
  matching_mode: "Strict" | "Loose";
  case_sensitivity: "CaseSensitive" | "CaseInsensitive";
  group_id: string | null;
  enabled: boolean;
  created_at: string;
  modified_at: string;
  last_used_at: string | null;
  ai_generated: boolean;
  image_data: string | null;
  content_type: "Text" | "Image" | "Both";
}

export interface Group {
  uuid: string;
  name: string;
  description: string;
  enabled: boolean;
  created_at: string;
  modified_at: string;
}

export interface ImportResult {
  snippets_imported: number;
  groups_imported: number;
  errors: string[];
}

export interface BackupInfo {
  filename: string;
  created_at: string;
  snippet_count: number;
  group_count: number;
  size_bytes: number;
}

export type Page = "snippets" | "chat" | "search" | "settings";
