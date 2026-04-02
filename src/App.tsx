import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save, open as openPicker } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { writeTextFile, readFile } from "@tauri-apps/plugin-fs";
import { getVersion } from "@tauri-apps/api/app";
import { useTranslation, Language } from "./i18n";
import { getPreferredTheme, setTheme, toggleTheme, getStoredTheme, initTheme, Theme } from "./theme";
import { Snippet, Group, Page, ImportResult } from "./types";
import { estimateTokenCount, truncateToTokens } from "./utils";
import SnippetEditor from "./components/SnippetEditor";
import SettingsPanel from "./components/SettingsPanel";

// ─── App ──────────────────────────────────────────────────────────────────────

export default function App() {
  const [page, setPage] = useState<Page>("snippets");
  const [ollamaOnline, setOllamaOnline] = useState(false);
  const [toast, setToast] = useState<{ msg: string; type: "success" | "error" } | null>(null);
  const [lang, setLang] = useState<Language>("both");
  const [appVersion, setAppVersion] = useState("...");
  const [showForm, setShowForm] = useState(false);
  const [editingSnippet, setEditingSnippet] = useState<Snippet | null>(null);
  const [theme, setThemeState] = useState<Theme>(() => getStoredTheme() ?? "dark");

  useEffect(() => {
    initTheme();
    const mq = window.matchMedia?.("(prefers-color-scheme: light)");
    const handler = () => {
      if (!getStoredTheme()) {
        const next = getPreferredTheme();
        setTheme(next);
        setThemeState(next);
      }
    };
    if (mq) mq.addEventListener("change", handler);

    const check = async () => {
      try {
        const online = await invoke<boolean>("ollama_status");
        setOllamaOnline(online);
      } catch { setOllamaOnline(false); }
    };
    check();
    const interval = setInterval(check, 15000);
    invoke<Language | null>("get_preference", { key: "language" })
      .then(v => { if (v) setLang(v); })
      .catch(() => {});
    getVersion().then(v => setAppVersion("v" + v)).catch(console.error);

    return () => {
      if (mq) mq.removeEventListener("change", handler);
      clearInterval(interval);
    };
  }, []);

  const t = useTranslation(lang);

  const showToast = useCallback((msgOrErr: any, type: "success" | "error" = "success") => {
    let msg = String(msgOrErr);
    if (msgOrErr instanceof Error) {
      msg = msgOrErr.message;
    } else if (msgOrErr && typeof msgOrErr === "object") {
      try { msg = JSON.stringify(msgOrErr); } catch { /* ignore */ }
    }

    if (type === "error") {
      console.error("BeefText Error UI:", msgOrErr);
      if (msg === "[object Object]") msg = "An unexpected error occurred (see console).";
      if (!msg || msg.trim() === "") msg = "Unknown error occurred.";
    }

    setToast({ msg, type });
    setTimeout(() => setToast(null), 3500);
  }, []);

  const handleToggleTheme = () => {
    const next = toggleTheme();
    setThemeState(next);
  };

  return (
    <div className="app-layout">
      <aside className="sidebar">
        <div className="sidebar-header">
          <div className="sidebar-logo">
            <div className="sidebar-logo-icon">⚡</div>
            <div>
              <div className="sidebar-logo-text">BeefText AI</div>
              <div className="sidebar-logo-version">{appVersion}</div>
            </div>
          </div>
          <button
            className="theme-toggle"
            onClick={handleToggleTheme}
            aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
            title={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
          >
            {theme === "dark" ? (
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <circle cx="12" cy="12" r="5"/>
                <line x1="12" y1="1" x2="12" y2="3"/>
                <line x1="12" y1="21" x2="12" y2="23"/>
                <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
                <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
                <line x1="1" y1="12" x2="3" y2="12"/>
                <line x1="21" y1="12" x2="23" y2="12"/>
                <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
                <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
              </svg>
            ) : (
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
              </svg>
            )}
          </button>
        </div>
        <nav className="sidebar-nav">
          <div className="nav-section-title">Main</div>
          <div className={`nav-item ${page === "snippets" ? "active" : ""}`} onClick={() => setPage("snippets")}>
            <span className="nav-item-icon">📋</span> {t("snippets")}
          </div>
          <div className={`nav-item ${page === "chat" ? "active" : ""}`} onClick={() => setPage("chat")}>
            <span className="nav-item-icon">🤖</span> {t("chat")}
          </div>
          <div className={`nav-item ${page === "search" ? "active" : ""}`} onClick={() => setPage("search")}>
            <span className="nav-item-icon">🔍</span> {t("search")}
          </div>
          <div className="nav-section-title">System</div>
          <div className={`nav-item ${page === "settings" ? "active" : ""}`} onClick={() => setPage("settings")}>
            <span className="nav-item-icon">⚙️</span> {t("settings")}
          </div>
        </nav>
        <div className="sidebar-status">
          <div className="status-indicator">
            <span className={`status-dot ${ollamaOnline ? "online" : "offline"}`} />
            Ollama: {ollamaOnline ? "Connected" : "Offline"}
          </div>
        </div>
      </aside>

      <main className="main-content">
        {page === "snippets" && <SnippetsPage showToast={showToast} showForm={showForm} setShowForm={setShowForm} editingSnippet={editingSnippet} setEditingSnippet={setEditingSnippet} t={t} />}
        {page === "chat" && <ChatPage showToast={showToast} ollamaOnline={ollamaOnline} />}
        {page === "search" && <SearchPage showToast={showToast} ollamaOnline={ollamaOnline} onEditSnippet={(s) => { setEditingSnippet(s); setShowForm(true); setPage("snippets"); }} />}
        {page === "settings" && <SettingsPanel showToast={showToast} ollamaOnline={ollamaOnline} onLanguageChange={setLang} />}
      </main>

      {toast && <div className={`toast ${toast.type}`}>{toast.msg}</div>}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Snippets Page — with Group sidebar
// ═══════════════════════════════════════════════════════════════════════════════

function SnippetsPage({ showToast, showForm, setShowForm, editingSnippet, setEditingSnippet }: {
  showToast: (m: string, t?: "success" | "error") => void;
  showForm: boolean;
  setShowForm: (v: boolean) => void;
  editingSnippet: Snippet | null;
  setEditingSnippet: (s: Snippet | null) => void;
  t: any;
}) {
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [groups, setGroups] = useState<Group[]>([]);
  const [filter, setFilter] = useState("");
  const [selectedGroup, setSelectedGroup] = useState<string | null>(null); // null = "All", "ungrouped" = no group
  const [showGroupForm, setShowGroupForm] = useState(false);
  const [editingGroup, setEditingGroup] = useState<Group | null>(null);
  const [showImport, setShowImport] = useState(false);

  const load = useCallback(async () => {
    try {
      const [s, g] = await Promise.all([
        invoke<Snippet[]>("get_snippets"),
        invoke<Group[]>("get_groups"),
      ]);
      setSnippets(s);
      setGroups(g);
    } catch (e) { console.error(e); }
  }, []);

  useEffect(() => { load(); }, [load]);

  const filtered = snippets.filter(s => {
    // Group filter
    if (selectedGroup === "ungrouped" && s.group_id !== null) return false;
    if (selectedGroup && selectedGroup !== "ungrouped" && s.group_id !== selectedGroup) return false;
    // Text filter
    if (!filter) return true;
    const q = filter.toLowerCase();
    return s.keyword.toLowerCase().includes(q) || s.name.toLowerCase().includes(q) || s.snippet.toLowerCase().includes(q) || s.description.toLowerCase().includes(q);
  });

  const handleToggle = async (uuid: string, enabled: boolean) => {
    try {
      await invoke("toggle_snippet_enabled", { uuid, enabled: !enabled });
      load();
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleDeleteGroup = async (uuid: string) => {
    const groupSnippetCount = snippets.filter(s => s.group_id === uuid).length;
    const mode = window.confirm(
      `Delete group "${groups.find(g => g.uuid === uuid)?.name}"?\n\n` +
      `Press OK: Delete group and all its snippets (${groupSnippetCount} snippet${groupSnippetCount !== 1 ? "s" : ""} will be deleted).\n` +
      `Press Cancel: Delete group only, keeping all snippets.`
    );
    try {
      await invoke("delete_group_cmd", { uuid, deleteSnippets: mode });
      setSelectedGroup(null);
      showToast(mode ? "Group and snippets deleted" : "Group deleted, snippets kept");
      load();
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleDeleteGroupSnippets = async (uuid: string) => {
    if (!window.confirm("Delete all snippets in this group?")) return;
    try {
      const count = await invoke<number>("delete_snippets_in_group_cmd", { groupUuid: uuid });
      showToast(`Deleted ${count} snippet(s)`);
      load();
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleExportJson = async () => {
    try {
      const path = await save({
        filters: [{ name: "JSON", extensions: ["json"] }],
        defaultPath: "beeftextai-export.json",
      });
      if (!path) return;
      const json = await invoke<string>("export_json");
      await writeTextFile(path, json);
      showToast(`Saved to ${path}`);
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleExportCsv = async () => {
    try {
      const path = await save({
        filters: [{ name: "CSV", extensions: ["csv"] }],
        defaultPath: "beeftextai-export.csv",
      });
      if (!path) return;
      const csv = await invoke<string>("export_csv");
      await writeTextFile(path, csv);
      showToast(`Saved to ${path}`);
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleCheatSheet = async () => {
    try {
      const path = await save({
        filters: [{ name: "HTML", extensions: ["html"] }],
        defaultPath: "cheat-sheet.html",
      });
      if (!path) return;
      const html = await invoke<string>("generate_cheat_sheet");
      await writeTextFile(path, html);
      await openPath(path);
      showToast(`Saved to ${path}`);
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleDeleteAll = async () => {
    if (!window.confirm("Are you sure you want to delete ALL snippets? This cannot be undone.")) return;
    try {
      await invoke("clear_all_data");
      showToast("All snippets deleted");
      load();
    } catch (e) { showToast(String(e), "error"); }
  };

  const groupSnippetCount = (gid: string | null) => snippets.filter(s => s.group_id === gid).length;

  return (
    <>
      <div className="content-header">
        <h1>📋 Snippets <span style={{ fontSize: 14, color: "var(--text-tertiary)", fontWeight: 400 }}>({filtered.length})</span></h1>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <div className="search-bar">
            <span className="search-bar-icon">🔍</span>
            <input placeholder="Search snippets..." value={filter} onChange={e => setFilter(e.target.value)} />
          </div>
          <div style={{ position: "relative" }}>
            <DropdownMenu items={[
              { label: "📥 Import from Beeftext", onClick: () => setShowImport(true) },
              { label: "📤 Export as JSON", onClick: handleExportJson },
              { label: "📤 Export as CSV", onClick: handleExportCsv },
              { label: "📄 Cheat Sheet", onClick: handleCheatSheet },
              { type: "separator" },
              { label: "🗑 Delete All Snippets", onClick: handleDeleteAll },
            ]} />
          </div>
          <button className="btn btn-primary" onClick={() => { setEditingSnippet(null); setShowForm(true); }}>
            + New Snippet
          </button>
        </div>
      </div>

      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        {/* Groups Sidebar */}
        <div className="groups-panel">
          <div className="groups-panel-header">
            <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.08em" }}>Groups</span>
            <button className="btn btn-sm btn-icon" style={{ padding: 4, fontSize: 16 }} onClick={() => { setEditingGroup(null); setShowGroupForm(true); }} title="New Group">+</button>
          </div>
          <div className="group-list">
            <div className={`group-item ${selectedGroup === null ? "active" : ""}`} onClick={() => setSelectedGroup(null)}>
              <span>📦 All Snippets</span>
              <span className="group-count">{snippets.length}</span>
            </div>
            <div className={`group-item ${selectedGroup === "ungrouped" ? "active" : ""}`} onClick={() => setSelectedGroup("ungrouped")}>
              <span>📄 Ungrouped</span>
              <span className="group-count">{groupSnippetCount(null)}</span>
            </div>
            {groups.map(g => (
              <div key={g.uuid} className={`group-item ${selectedGroup === g.uuid ? "active" : ""}`}>
                <span onClick={() => setSelectedGroup(g.uuid)} style={{ flex: 1, cursor: "pointer" }}>
                  📁 {g.name}
                </span>
                <span className="group-count">{groupSnippetCount(g.uuid)}</span>
                <div className="group-actions">
                  <button className="group-action-btn" onClick={(e) => { e.stopPropagation(); setEditingGroup(g); setShowGroupForm(true); }} title="Edit">✏️</button>
                  <button className="group-action-btn" onClick={(e) => { e.stopPropagation(); handleDeleteGroupSnippets(g.uuid); }} title="Delete All Snippets">🗑️</button>
                  <button className="group-action-btn" onClick={(e) => { e.stopPropagation(); handleDeleteGroup(g.uuid); }} title="Delete"><span className="btn-delete-icon">×</span></button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Snippets Table */}
        <div className="content-body" style={{ flex: 1 }}>
          {filtered.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state-icon">📝</div>
              <h3>{snippets.length === 0 ? "No snippets yet" : "No results"}</h3>
              <p>{snippets.length === 0 ? "Create your first text snippet or ask the AI chatbot to generate one for you!" : "Try a different search term or group"}</p>
              {snippets.length === 0 && (
                <button className="btn btn-primary" onClick={() => { setEditingSnippet(null); setShowForm(true); }}>
                  + Create First Snippet
                </button>
              )}
            </div>
          ) : (
            <table className="snippet-table">
              <thead>
                <tr>
                  <th style={{ width: 40 }}></th>
                  <th>Keyword</th>
                  <th>Name</th>
                  <th>Snippet</th>
                  <th>Group</th>
                  <th>Mode</th>
                  <th style={{ width: 80 }}>Actions</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map(s => (
                  <tr key={s.uuid} className={`snippet-row ${!s.enabled ? "disabled" : ""}`} onClick={() => { setEditingSnippet(s); setShowForm(true); }}>
                    <td>
                      <button
                        className={`toggle-btn ${s.enabled ? "on" : "off"}`}
                        onClick={(e) => { e.stopPropagation(); handleToggle(s.uuid, s.enabled); }}
                        title={s.enabled ? "Enabled" : "Disabled"}
                      >
                        {s.enabled ? "✅" : "⬜"}
                      </button>
                    </td>
                    <td><span className="keyword-badge">{s.keyword}</span></td>
                    <td>
                      {s.name || <span style={{ color: "var(--text-tertiary)" }}>Untitled</span>}
                      {s.ai_generated && <span className="ai-badge" style={{ marginLeft: 8 }}>🤖 AI</span>}
                    </td>
                    <td><span className="snippet-preview">{s.snippet}</span></td>
                    <td style={{ color: "var(--text-tertiary)", fontSize: 12 }}>
                      {groups.find(g => g.uuid === s.group_id)?.name || "—"}
                    </td>
                    <td style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
                      {s.matching_mode === "Loose" ? "Loose" : "Strict"}
                      {s.case_sensitivity === "CaseInsensitive" ? " · CI" : ""}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>

      {showForm && (
        <SnippetEditor snippet={editingSnippet} groups={groups} onClose={() => setShowForm(false)}
          onSave={() => { setShowForm(false); load(); showToast(editingSnippet ? "Snippet updated" : "Snippet created"); }}
          showToast={showToast} t={t} />
      )}

      {showGroupForm && (
        <GroupModal group={editingGroup} onClose={() => setShowGroupForm(false)}
          onSave={() => { setShowGroupForm(false); load(); showToast(editingGroup ? "Group updated" : "Group created"); }}
          showToast={showToast} />
      )}

      {showImport && (
        <ImportModal onClose={() => setShowImport(false)}
          onImport={() => { setShowImport(false); load(); }}
          showToast={showToast} />
      )}
    </>
  );
}

// ─── Dropdown Menu ────────────────────────────────────────────────────────────

type DropdownItem = { label?: string; onClick?: () => void; type?: "separator" };

function DropdownMenu({ items }: { items: DropdownItem[] }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  return (
    <div ref={ref} style={{ position: "relative" }}>
      <button className="btn btn-secondary btn-sm" onClick={() => setOpen(!open)}>⋮ More</button>
      {open && (
        <div className="dropdown-menu">
          {items.map((item, i) => (
            item.type === "separator" ? (
              <div key={i} className="dropdown-separator" />
            ) : (
              <button key={i} className="dropdown-item" onClick={() => { item.onClick?.(); setOpen(false); }}>
                {item.label}
              </button>
            )
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Group Modal ──────────────────────────────────────────────────────────────

function GroupModal({ group, onClose, onSave, showToast }: {
  group: Group | null; onClose: () => void; onSave: () => void;
  showToast: (m: string, t?: "success" | "error") => void;
}) {
  const [name, setName] = useState(group?.name || "");
  const [desc, setDesc] = useState(group?.description || "");
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!name.trim()) { showToast("Group name is required", "error"); return; }
    setSaving(true);
    try {
      if (group) {
        await invoke("update_group_cmd", { g: { ...group, name, description: desc, modified_at: new Date().toISOString() } });
      } else {
        await invoke("add_group_cmd", { name, description: desc });
      }
      onSave();
    } catch (e) { showToast(String(e), "error"); }
    finally { setSaving(false); }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" style={{ maxWidth: 440 }} onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{group ? "✏️ Edit Group" : "📁 New Group"}</h2>
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          <div className="input-group">
            <label className="input-label">Group Name</label>
            <input className="input" placeholder="e.g. Email Templates" value={name} onChange={e => setName(e.target.value)} autoFocus />
          </div>
          <div className="input-group">
            <label className="input-label">Description</label>
            <input className="input" placeholder="Optional description" value={desc} onChange={e => setDesc(e.target.value)} />
          </div>
        </div>
        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onClose}>Cancel</button>
          <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
            {saving ? <span className="spinner" /> : null} {group ? "Save" : "Create Group"}
          </button>
        </div>
      </div>
    </div>
  );
}

// ─── Import Modal ─────────────────────────────────────────────────────────────

function ImportModal({ onClose, onImport, showToast }: {
  onClose: () => void; onImport: () => void;
  showToast: (m: string, t?: "success" | "error") => void;
}) {
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);

  const handleFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = async (ev) => {
      const content = ev.target?.result as string;
      setImporting(true);
      try {
        const res = await invoke<ImportResult>("import_beeftext", { jsonContent: content });
        setResult(res);
        showToast(`Imported ${res.snippets_imported} snippets and ${res.groups_imported} groups`);
        onImport();
      } catch (e) { showToast(String(e), "error"); }
      finally { setImporting(false); }
    };
    reader.readAsText(file);
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" style={{ maxWidth: 480 }} onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>📥 Import from Beeftext</h2>
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          {!result ? (
            <>
              <p style={{ fontSize: 14, color: "var(--text-secondary)", marginBottom: 16 }}>
                Select your Beeftext backup file (usually <code style={{ color: "var(--accent-secondary)", fontSize: 12 }}>comboList.json</code> from <code style={{ color: "var(--accent-secondary)", fontSize: 12 }}>%AppData%\Beeftext\</code>)
              </p>
              <label className="file-upload-label">
                <input type="file" accept="*" onChange={handleFile} style={{ display: "none" }} />
                <div className="file-upload-area">
                  {importing ? <span className="spinner" /> : "📂"}
                  <span>{importing ? "Importing..." : "Click to select file"}</span>
                </div>
              </label>
            </>
          ) : (
            <div>
              <div style={{ display: "flex", gap: 16, marginBottom: 16 }}>
                <div className="stat-card">
                  <div className="stat-value">{result.snippets_imported}</div>
                  <div className="stat-label">Snippets</div>
                </div>
                <div className="stat-card">
                  <div className="stat-value">{result.groups_imported}</div>
                  <div className="stat-label">Groups</div>
                </div>
              </div>
              {result.errors.length > 0 && (
                <div style={{ fontSize: 12, color: "var(--accent-warning)" }}>
                  <strong>⚠️ {result.errors.length} warnings:</strong>
                  <ul style={{ marginTop: 4 }}>{result.errors.slice(0, 5).map((e: string, i: number) => <li key={i}>{e}</li>)}</ul>
                </div>
              )}
            </div>
          )}
        </div>
        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onClose}>{result ? "Done" : "Cancel"}</button>
        </div>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Chat Page
// ═══════════════════════════════════════════════════════════════════════════════

function ChatPage({ showToast, ollamaOnline }: { showToast: (m: string, t?: "success" | "error") => void; ollamaOnline: boolean }) {
  const [messages, setMessages] = useState<{ role: string; content: string; imagePreview?: string }[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [imageData, setImageData] = useState<string | null>(null);
  const [imagePreview, setImagePreview] = useState<string | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const chatContainerRef = useRef<HTMLDivElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // HTML5 drag-drop handler for external files (Windows Explorer drag)
  useEffect(() => {
    const handleDocDragOver = (e: DragEvent) => {
      e.preventDefault();
      if (e.dataTransfer?.types.includes("Files")) {
        if (ollamaOnline && !loading) setIsDragging(true);
      }
    };
    const handleDocDrop = (e: DragEvent) => {
      e.preventDefault();
      setIsDragging(false);
      if (!ollamaOnline || loading) return;
      const files = e.dataTransfer?.files;
      if (files && files.length > 0) {
        const file = files[0];
        if (file.type.startsWith("image/")) {
          const reader = new FileReader();
          reader.onload = (ev) => {
            const result = ev.target?.result as string;
            setImageData(result);
            setImagePreview(result);
          };
          reader.readAsDataURL(file);
        }
      }
    };
    const handleDocDragLeave = (e: DragEvent) => {
      if (e.relatedTarget === null) setIsDragging(false);
    };

    // Native Tauri v2 drag-drop listener (reliable for OS-level drops)
    const unlisten = listen<{ paths: string[] }>("tauri://drag-drop", async (event) => {
      setIsDragging(false);
      if (!ollamaOnline || loading) return;
      const paths = event.payload.paths;
      if (paths && paths.length > 0) {
        const path = paths[0];
        const lower = path.toLowerCase();
        if (lower.endsWith(".png") || lower.endsWith(".jpg") || lower.endsWith(".jpeg") || lower.endsWith(".gif") || lower.endsWith(".webp")) {
          try {
            const content = await readFile(path);
            const ext = path.split('.').pop()?.toLowerCase() || 'png';
            const mime = ext === 'jpg' ? 'image/jpeg' : `image/${ext}`;
            const blob = new Blob([content], { type: mime });
            const reader = new FileReader();
            reader.onload = (ev) => {
              const result = ev.target?.result as string;
              setImageData(result);
              setImagePreview(result);
            };
            reader.readAsDataURL(blob);
          } catch (e) { console.error("Error reading dropped file:", e); }
        }
      }
    });

    document.addEventListener("dragover", handleDocDragOver);
    document.addEventListener("drop", handleDocDrop);
    document.addEventListener("dragleave", handleDocDragLeave);
    return () => {
      document.removeEventListener("dragover", handleDocDragOver);
      document.removeEventListener("drop", handleDocDrop);
      document.removeEventListener("dragleave", handleDocDragLeave);
      unlisten.then(f => f());
    };
  }, [ollamaOnline, loading]);

  useEffect(() => {
    invoke<[string, string][]>("get_chat_history_cmd").then(history => {
      setMessages(history.map(([role, content]) => ({ role, content })));
    }).catch(() => {});
  }, []);

  useEffect(() => { messagesEndRef.current?.scrollIntoView({ behavior: "smooth" }); }, [messages]);

  // Handle paste (Ctrl+V) for images
  useEffect(() => {
    const handlePaste = (e: ClipboardEvent) => {
      if (!ollamaOnline || loading) return;
      const items = e.clipboardData?.items;
      if (!items) return;
      for (const item of items) {
        if (item.type.startsWith("image/")) {
          e.preventDefault();
          const file = item.getAsFile();
          if (file) {
            const reader = new FileReader();
            reader.onload = (ev) => {
              const result = ev.target?.result as string;
              setImageData(result);
              setImagePreview(result);
            };
            reader.readAsDataURL(file);
          }
          break;
        }
      }
    };
    const chatEl = chatContainerRef.current;
    chatEl?.addEventListener("paste", handlePaste);
    return () => chatEl?.removeEventListener("paste", handlePaste);
  }, [ollamaOnline, loading]);

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    if (ollamaOnline && !loading) setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    if (!ollamaOnline || loading) return;
    const files = e.dataTransfer?.files;
    if (files && files.length > 0) {
      const file = files[0];
      if (file.type.startsWith("image/")) {
        const reader = new FileReader();
        reader.onload = (ev) => {
          const result = ev.target?.result as string;
          setImageData(result);
          setImagePreview(result);
        };
        reader.readAsDataURL(file);
      }
    }
  };

  const handleImageSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const result = ev.target?.result as string;
      setImageData(result);
      setImagePreview(result);
    };
    reader.readAsDataURL(file);
  };

  const sendMessage = async () => {
    // Require text input always — image-only is rejected (model is non-OCR)
    if (!input.trim() || loading) {
      if (imageData && !input.trim()) {
        showToast("Tambahkan teks dulu sebelum kirim gambar.", "error");
      }
      return;
    }
    const MAX_INPUT_TOKENS = 2000;
    const rawMsg = input.trim();
    const userMsg = rawMsg ? truncateToTokens(rawMsg, MAX_INPUT_TOKENS) : "";
    const wasTruncated = userMsg.length < rawMsg.length;
    void wasTruncated; // suppress unused warning
    const sentImagePreview = imagePreview; // capture before clearing
    setInput("");
    // Add user message + keep only last 10 messages to reduce backend load
    setMessages(prev => {
      const updated = [...prev, {
        role: "user",
        content: userMsg,
        imagePreview: sentImagePreview ?? undefined,
      }];
      return updated.length > 10 ? updated.slice(updated.length - 10) : updated;
    });
    setLoading(true);

    const backendMsg = sentImagePreview 
      ? userMsg + "\n\n[SYSTEM NOTE: The user has successfully attached an image to this request. Acknowledge it, and generate the JSON snippet with content_type 'Image' or 'Both'. OMIT the 'image_data' field from the JSON. Do NOT say you cannot see the image.]"
      : userMsg;

    try {
      const response = await invoke<string>("chat_with_ai", { message: backendMsg, imageData });
      setMessages(prev => [...prev, { role: "assistant", content: response, imagePreview: sentImagePreview ?? undefined }]);
    } catch (e) {
      showToast(String(e), "error");
      setMessages(prev => [...prev, { role: "assistant", content: `❌ Error: ${e}` }]);
    } finally {
      setLoading(false);
      setImageData(null);
      setImagePreview(null);
    }
  };

  const handleClear = async () => {
    try { await invoke("clear_chat"); setMessages([]); showToast("Chat cleared"); }
    catch (e) { showToast(String(e), "error"); }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendMessage(); }
  };

  return (
    <div
      ref={chatContainerRef}
      className="chat-container"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      style={{ position: "relative" }}
    >
      {isDragging && (
        <div style={{
          position: "absolute",
          inset: 0,
          background: "rgba(0, 212, 170, 0.15)",
          border: "3px dashed var(--accent-primary)",
          borderRadius: 8,
          zIndex: 10,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontSize: 24,
          fontWeight: "bold",
          color: "var(--accent-primary)",
          pointerEvents: "none"
        }}>
          🖼️ Drop image here
        </div>
      )}
      <div className="content-header">
        <h1>🤖 AI Chat</h1>
        <button className="btn btn-secondary btn-sm" onClick={handleClear}>🗑 Clear Chat</button>
      </div>
      <div className="chat-messages">
        {messages.length === 0 && (
          <div className="empty-state">
            <div className="empty-state-icon">💬</div>
            <h3>Start a conversation</h3>
            <p>Ask the AI to create snippets, suggest keywords, or help organize your text library.<br />Try: "Buatkan snippet email izin sakit"<br /><br /><small style={{ opacity: 0.6 }}>💡 Attach images via drag &amp; drop or Ctrl+V</small></p>
          </div>
        )}
        {messages.map((msg, i) => (
          <div key={i} className={`chat-message ${msg.role}`}>
            <div className={`chat-avatar ${msg.role === "user" ? "user-avatar" : "ai-avatar"}`}>
              {msg.role === "user" ? "👤" : "🤖"}
            </div>
            <div className="chat-bubble">
              {msg.imagePreview && (
                <img
                  src={msg.imagePreview}
                  alt="Attachment"
                  style={{ maxHeight: 120, maxWidth: "100%", borderRadius: 6, marginBottom: 8, display: "block", border: "1px solid var(--border)" }}
                />
              )}
              <MessageContent content={msg.content} showToast={showToast} />
            </div>
          </div>
        ))}
        {loading && (
          <div className="chat-message assistant">
            <div className="chat-avatar ai-avatar">🤖</div>
            <div className="chat-bubble" style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span className="spinner" /> Thinking...
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>
      <div className="chat-input-container">
        {!ollamaOnline && (
          <div style={{ color: "var(--accent-warning)", fontSize: 12, marginBottom: 8, display: "flex", alignItems: "center", gap: 6 }}>
            ⚠️ Ollama is offline. Please start Ollama to use the AI chat.
          </div>
        )}
        {imagePreview && (
          <div style={{ position: "relative", display: "inline-block", marginBottom: 8 }}>
            <img src={imagePreview} alt="Attachment" style={{ maxHeight: 80, borderRadius: 6, border: "1px solid var(--border)" }} />
            <button onClick={() => { setImageData(null); setImagePreview(null); }}
              style={{ position: "absolute", top: -8, right: -8, background: "var(--bg-tertiary)", border: "none", borderRadius: "50%", width: 20, height: 20, cursor: "pointer", fontSize: 10, lineHeight: "20px" }}>✕</button>
          </div>
        )}
        <div className="chat-input-wrapper">
          <input type="file" accept="image/*" onChange={handleImageSelect} disabled={!ollamaOnline || loading} style={{ display: "none" }} id="chat-image-upload" />
          <label htmlFor="chat-image-upload" style={{ cursor: "pointer", padding: "4px 8px", fontSize: 18 }} title="Attach image">🖼️</label>
          <textarea className="chat-input" placeholder="Ask AI to create a snippet..." value={input} onChange={e => setInput(e.target.value)} onKeyDown={handleKeyDown} rows={1} disabled={!ollamaOnline || loading} />
          <button className="chat-send-btn" onClick={sendMessage} disabled={!ollamaOnline || loading || !input.trim()}>➤</button>
        </div>
      </div>
    </div>
  );
}

// ─── Inline Markdown Renderer ─────────────────────────────────────────────────
// Parsing priority (CRITICAL — do not reorder):
//   1. Triple-backtick code blocks (``` ... ```)
//   2. Single-backtick inline code (` ... `)
//   3. Bold (**text**)
//   4. Italic (*text*)
//   5. Unordered / ordered lists
//   6. Line breaks
// TODO: Tables and deeply nested formatting are out of scope for now.

function renderMarkdown(text: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  let key = 0;

  // Split on triple-backtick code blocks first
  const codeBlockParts = text.split(/(```[\s\S]*?```)/g);

  for (const part of codeBlockParts) {
    // ── Triple-backtick code block ──
    if (part.startsWith("```") && part.endsWith("```")) {
      const inner = part.slice(3, -3);
      // Strip optional language hint on first line (e.g. ```json ...)
      const firstNewline = inner.indexOf("\n");
      const code = firstNewline !== -1 ? inner.slice(firstNewline + 1) : inner;
      nodes.push(
        <pre key={key++} className="chat-code-block"><code>{code}</code></pre>
      );
      continue;
    }

    // ── Line-by-line parsing ──
    const lines = part.split("\n");
    let i = 0;
    while (i < lines.length) {
      const line = lines[i];

      // Unordered list item
      if (/^[-*]\s/.test(line)) {
        const listItems: React.ReactNode[] = [];
        while (i < lines.length && /^[-*]\s/.test(lines[i])) {
          listItems.push(<li key={key++}>{renderInline(lines[i].replace(/^[-*]\s/, ""), key)}</li>);
          i++;
        }
        nodes.push(<ul key={key++} className="chat-md-list">{listItems}</ul>);
        continue;
      }

      // Ordered list item
      if (/^\d+\.\s/.test(line)) {
        const listItems: React.ReactNode[] = [];
        while (i < lines.length && /^\d+\.\s/.test(lines[i])) {
          listItems.push(<li key={key++}>{renderInline(lines[i].replace(/^\d+\.\s/, ""), key)}</li>);
          i++;
        }
        nodes.push(<ol key={key++} className="chat-md-list">{listItems}</ol>);
        continue;
      }

      // Empty line → spacer
      if (line.trim() === "") {
        nodes.push(<br key={key++} />);
        i++;
        continue;
      }

      // Normal paragraph line
      nodes.push(<span key={key++}>{renderInline(line, key)}<br /></span>);
      i++;
    }
  }

  return nodes;
}

/** Render inline markdown: single backtick, bold, italic */
function renderInline(text: string, baseKey: number): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  let key = baseKey * 1000;

  // Split on single-backtick inline code (after triple-backtick already removed)
  const inlineParts = text.split(/(`[^`]+`)/g);

  for (const part of inlineParts) {
    if (part.startsWith("`") && part.endsWith("`") && part.length > 2) {
      nodes.push(<code key={key++} className="chat-inline-code">{part.slice(1, -1)}</code>);
      continue;
    }

    // Process bold + italic on the remaining plain text
    let remaining = part;
    const boldItalicRegex = /(\*\*[^*]+\*\*|\*[^*]+\*)/g;
    let lastIndex = 0;
    let match: RegExpExecArray | null;

    while ((match = boldItalicRegex.exec(remaining)) !== null) {
      if (match.index > lastIndex) {
        nodes.push(<span key={key++}>{remaining.slice(lastIndex, match.index)}</span>);
      }
      const raw = match[0];
      if (raw.startsWith("**")) {
        nodes.push(<strong key={key++}>{raw.slice(2, -2)}</strong>);
      } else {
        nodes.push(<em key={key++}>{raw.slice(1, -1)}</em>);
      }
      lastIndex = match.index + raw.length;
    }
    if (lastIndex < remaining.length) {
      nodes.push(<span key={key++}>{remaining.slice(lastIndex)}</span>);
    }
  }

  return nodes;
}

// ─── Message Content ──────────────────────────────────────────────────────────

function MessageContent({ content, showToast }: { content: string; showToast: (m: string, t?: "success" | "error") => void }) {
  const snippetJson = extractSnippetJson(content);
  if (snippetJson) {
    const textBefore = content.substring(0, content.indexOf("{"));
    const textAfter = content.substring(content.lastIndexOf("}") + 1);
    const handleSave = async () => {
      try {
        const generatedKeyword = snippetJson.keyword || "//new";
        const allSnippets = await invoke<Snippet[]>("get_snippets");
        const isDuplicate = allSnippets.some(s => s.keyword === generatedKeyword);
        if (isDuplicate) {
          if (!window.confirm(`Are you sure? The keyword '${generatedKeyword}' has already been used for another snippet.`)) {
            return;
          }
        }

        let groupId: string | null = null;
        if (snippetJson.group) {
          const allGroups = await invoke<Group[]>("get_groups");
          const existingGroup = allGroups.find(g => g.name.toLowerCase() === snippetJson.group.toLowerCase());
          if (existingGroup) {
            groupId = existingGroup.uuid;
          } else {
            const newGroup = await invoke<Group>("add_group_cmd", { name: snippetJson.group, description: "Auto-generated group" });
            groupId = newGroup.uuid;
          }
        }

        await invoke("add_snippet", { keyword: generatedKeyword, snippetText: snippetJson.snippet || "", name: snippetJson.name || "", description: snippetJson.description || "", groupId: groupId, aiGenerated: true, imageData: snippetJson.image_data || null, contentType: snippetJson.content_type || "Text" });
        showToast("✅ Snippet saved!");
      } catch (e) { showToast(String(e), "error"); }
    };
    const ct = snippetJson.content_type;
    const img = snippetJson.image_data;
    return (
      <div>
        {textBefore && <p style={{ marginBottom: 10 }}>{textBefore}</p>}
        <div className="snippet-card-chat">
          <div className="snippet-card-chat-header">📋 Generated Snippet</div>
          {snippetJson.keyword && <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Keyword:</span><span className="keyword-badge">{snippetJson.keyword}</span></div>}
          {snippetJson.name && <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Name:</span><span className="snippet-card-chat-value">{snippetJson.name}</span></div>}
          {snippetJson.description && <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Desc:</span><span className="snippet-card-chat-value" style={{ color: "var(--text-secondary)", fontSize: 12 }}>{snippetJson.description}</span></div>}
          {snippetJson.group && <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Group:</span><span className="snippet-card-chat-value" style={{ color: "var(--text-secondary)", fontSize: 12 }}>{snippetJson.group}</span></div>}
          {snippetJson.snippet && <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Snippet:</span><span className="snippet-card-chat-value" style={{ whiteSpace: "pre-wrap" }}>{snippetJson.snippet}</span></div>}
          {img && (ct === "Image" || ct === "Both") && (
            <div className="snippet-card-chat-field">
              <span className="snippet-card-chat-label">Image Preview:</span>
              <img src={img} alt="Snippet" style={{ maxHeight: 100, borderRadius: 4, marginTop: 4, border: "1px solid var(--border)" }} />
            </div>
          )}
          <div className="snippet-card-chat-actions"><button className="btn btn-primary btn-sm" onClick={handleSave}>✅ Save Snippet</button></div>
        </div>
        {textAfter && <p style={{ marginTop: 10 }}>{textAfter}</p>}
      </div>
    );
  }
  return <div className="chat-md">{renderMarkdown(content)}</div>;
}

function extractSnippetJson(text: string): any {
  try {
    const match = text.match(/\{[^{}]*"keyword"[^{}]*\}/s) || text.match(/\{[^{}]*"snippet"[^{}]*\}/s);
    if (match) return JSON.parse(match[0]);
  } catch {}
  return null;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Search Page (OmniSearch)
// ═══════════════════════════════════════════════════════════════════════════════

function SearchPage({ showToast, ollamaOnline, onEditSnippet }: { showToast: (m: string, t?: "success" | "error") => void; ollamaOnline: boolean; onEditSnippet: (snippet: Snippet) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<(Snippet & { score?: number })[]>([]);
  const [searching, setSearching] = useState(false);
  const [allSnippets, setAllSnippets] = useState<Snippet[]>([]);

  useEffect(() => { invoke<Snippet[]>("get_snippets").then(setAllSnippets).catch(() => {}); }, []);

  const handleSearch = async () => {
    if (!query.trim()) return;
    setSearching(true);
    try {
      const keywordResults = allSnippets.filter(s => {
        const q = query.toLowerCase();
        return s.keyword.toLowerCase().includes(q) || s.name.toLowerCase().includes(q) || s.snippet.toLowerCase().includes(q);
      });
      let semanticResults: (Snippet & { score: number })[] = [];
      if (ollamaOnline) {
        try {
          const scores = await invoke<[string, number][]>("semantic_search", { query, limit: 10 });
          semanticResults = scores.map(([uuid, score]) => {
            const snippet = allSnippets.find(s => s.uuid === uuid);
            return snippet ? { ...snippet, score } : null;
          }).filter(Boolean) as any;
        } catch {}
      }
      const semanticUuids = new Set(semanticResults.map(r => r.uuid));
      const combined = [...semanticResults, ...keywordResults.filter(r => !semanticUuids.has(r.uuid)).map(r => ({ ...r, score: undefined }))];
      setResults(combined);
    } catch (e) { showToast(String(e), "error"); }
    finally { setSearching(false); }
  };

  return (
    <>
      <div className="content-header"><h1>🔍 OmniSearch</h1></div>
      <div className="content-body">
        <div style={{ display: "flex", gap: 10, marginBottom: 24 }}>
          <div className="search-bar" style={{ maxWidth: "none", flex: 1 }}>
            <span className="search-bar-icon">🔍</span>
            <input placeholder="Search by keyword, name, or describe what you need..." value={query} onChange={e => setQuery(e.target.value)} onKeyDown={e => e.key === "Enter" && handleSearch()} autoFocus />
          </div>
          <button className="btn btn-primary" onClick={handleSearch} disabled={searching}>
            {searching ? <span className="spinner" /> : "Search"}
          </button>
        </div>
        {!ollamaOnline && <div style={{ color: "var(--accent-warning)", fontSize: 13, marginBottom: 16 }}>⚠️ Ollama offline — Semantic search disabled. Only keyword matching is available.</div>}
        {results.length > 0 ? (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {results.map((s, i) => (
              <div key={s.uuid} className="card" style={{ padding: 16, display: "flex", alignItems: "center", gap: 16, cursor: "pointer" }} onClick={(e) => { e.stopPropagation(); onEditSnippet(s); }}>
                <div style={{ fontSize: 20, opacity: 0.5, width: 28, textAlign: "center", pointerEvents: "none" }}>{i + 1}</div>
                <div style={{ flex: 1, pointerEvents: "none" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 4 }}>
                    <span className="keyword-badge">{s.keyword}</span>
                    <span style={{ fontWeight: 600 }}>{s.name || "Untitled"}</span>
                    {s.ai_generated && <span className="ai-badge">🤖 AI</span>}
                  </div>
                  <div style={{ fontSize: 13, color: "var(--text-secondary)" }}>{s.snippet.substring(0, 120)}{s.snippet.length > 120 ? "..." : ""}</div>
                </div>
                {s.score !== undefined && (
                  <div style={{ textAlign: "right", minWidth: 60, pointerEvents: "none" }}>
                    <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>Relevance</div>
                    <div style={{ fontSize: 14, fontWeight: 700, color: s.score > 0.7 ? "var(--accent-success)" : "var(--text-secondary)" }}>{Math.round(s.score * 100)}%</div>
                  </div>
                )}
                <button
                  style={{ pointerEvents: "auto", padding: "6px 12px", background: "var(--accent-primary)", color: "#fff", border: "none", borderRadius: "var(--radius-md)", fontSize: 12, fontWeight: 600, cursor: "pointer", whiteSpace: "nowrap" }}
                  onClick={(e) => { e.stopPropagation(); e.preventDefault(); onEditSnippet(s); }}
                >
                  ✏️ Edit
                </button>
              </div>
            ))}
          </div>
        ) : query && !searching ? (
          <div className="empty-state"><div className="empty-state-icon">🔎</div><h3>No results found</h3><p>Try a different search query or create a new snippet.</p></div>
        ) : !query ? (
          <div className="empty-state"><div className="empty-state-icon">✨</div><h3>Smart Search</h3><p>Search by exact keyword, name, or use natural language descriptions. AI-powered semantic search finds the most relevant snippets.</p></div>
        ) : null}
      </div>
    </>
  );
}
