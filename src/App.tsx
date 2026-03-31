import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useTranslation, Language } from "./i18n";

// ─── Types ────────────────────────────────────────────────────────────────────

interface Snippet {
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
}

interface Group {
  uuid: string;
  name: string;
  description: string;
  enabled: boolean;
  created_at: string;
  modified_at: string;
}

interface ImportResult {
  snippets_imported: number;
  groups_imported: number;
  errors: string[];
}

interface BackupInfo {
  filename: string;
  created_at: string;
  snippet_count: number;
  group_count: number;
  size_bytes: number;
}

type Page = "snippets" | "chat" | "search" | "settings";

// ─── App ──────────────────────────────────────────────────────────────────────

export default function App() {
  const [page, setPage] = useState<Page>("snippets");
  const [ollamaOnline, setOllamaOnline] = useState(false);
  const [toast, setToast] = useState<{ msg: string; type: "success" | "error" } | null>(null);
  const [lang, setLang] = useState<Language>("both");

  useEffect(() => {
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
    return () => clearInterval(interval);
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

  return (
    <div className="app-layout">
      <aside className="sidebar">
        <div className="sidebar-header">
          <div className="sidebar-logo">
            <div className="sidebar-logo-icon">⚡</div>
            <div>
              <div className="sidebar-logo-text">BeefText AI</div>
              <div className="sidebar-logo-version">v0.1.1</div>
            </div>
          </div>
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
        {page === "snippets" && <SnippetsPage showToast={showToast} />}
        {page === "chat" && <ChatPage showToast={showToast} ollamaOnline={ollamaOnline} />}
        {page === "search" && <SearchPage showToast={showToast} ollamaOnline={ollamaOnline} />}
        {page === "settings" && <SettingsPage showToast={showToast} ollamaOnline={ollamaOnline} />}
      </main>

      {toast && <div className={`toast ${toast.type}`}>{toast.msg}</div>}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Snippets Page — with Group sidebar
// ═══════════════════════════════════════════════════════════════════════════════

function SnippetsPage({ showToast }: { showToast: (m: string, t?: "success" | "error") => void }) {
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [groups, setGroups] = useState<Group[]>([]);
  const [filter, setFilter] = useState("");
  const [selectedGroup, setSelectedGroup] = useState<string | null>(null); // null = "All", "ungrouped" = no group
  const [showForm, setShowForm] = useState(false);
  const [editing, setEditing] = useState<Snippet | null>(null);
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

  const handleDelete = async (uuid: string) => {
    try {
      await invoke("delete_snippet_cmd", { uuid });
      showToast("Snippet deleted");
      load();
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleToggle = async (uuid: string, enabled: boolean) => {
    try {
      await invoke("toggle_snippet_enabled", { uuid, enabled: !enabled });
      load();
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleDeleteGroup = async (uuid: string) => {
    try {
      await invoke("delete_group_cmd", { uuid });
      setSelectedGroup(null);
      showToast("Group deleted");
      load();
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleExportJson = async () => {
    try {
      const json = await invoke<string>("export_json");
      downloadFile("beeftextai-export.json", json, "application/json");
      showToast("Exported as JSON");
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleExportCsv = async () => {
    try {
      const csv = await invoke<string>("export_csv");
      downloadFile("beeftextai-export.csv", csv, "text/csv");
      showToast("Exported as CSV");
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleCheatSheet = async () => {
    try {
      const html = await invoke<string>("generate_cheat_sheet");
      const blob = new Blob([html], { type: "text/html" });
      const url = URL.createObjectURL(blob);
      window.open(url, "_blank");
      showToast("Cheat sheet generated");
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
            ]} />
          </div>
          <button className="btn btn-primary" onClick={() => { setEditing(null); setShowForm(true); }}>
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
                  <button className="group-action-btn" onClick={(e) => { e.stopPropagation(); handleDeleteGroup(g.uuid); }} title="Delete">🗑</button>
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
                <button className="btn btn-primary" onClick={() => { setEditing(null); setShowForm(true); }}>
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
                  <tr key={s.uuid} className={`snippet-row ${!s.enabled ? "disabled" : ""}`} onClick={() => { setEditing(s); setShowForm(true); }}>
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
                    <td>
                      <button className="btn btn-danger btn-sm btn-icon" onClick={(e) => { e.stopPropagation(); handleDelete(s.uuid); }} title="Delete">
                        🗑
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>

      {showForm && (
        <SnippetModal snippet={editing} groups={groups} onClose={() => setShowForm(false)}
          onSave={() => { setShowForm(false); load(); showToast(editing ? "Snippet updated" : "Snippet created"); }}
          showToast={showToast} />
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

function DropdownMenu({ items }: { items: { label: string; onClick: () => void }[] }) {
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
            <button key={i} className="dropdown-item" onClick={() => { item.onClick(); setOpen(false); }}>
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Snippet Modal ────────────────────────────────────────────────────────────

function SnippetModal({ snippet, groups, onClose, onSave, showToast }: {
  snippet: Snippet | null; groups: Group[]; onClose: () => void; onSave: () => void;
  showToast: (m: string, t?: "success" | "error") => void;
}) {
  const [keyword, setKeyword] = useState(snippet?.keyword || "");
  const [name, setName] = useState(snippet?.name || "");
  const [text, setText] = useState(snippet?.snippet || "");
  const [desc, setDesc] = useState(snippet?.description || "");
  const [groupId, setGroupId] = useState<string | null>(snippet?.group_id || null);
  const [matchingMode, setMatchingMode] = useState<"Strict" | "Loose">(snippet?.matching_mode || "Strict");
  const [caseSensitivity, setCaseSensitivity] = useState<"CaseSensitive" | "CaseInsensitive">(snippet?.case_sensitivity || "CaseSensitive");
  const [saving, setSaving] = useState(false);

  const insertVariable = (v: string) => {
    setText(prev => prev + v);
  };

  const handleSave = async () => {
    if (!keyword.trim() || !text.trim()) { showToast("Keyword and snippet text are required", "error"); return; }
    
    // Check for duplicate keyword
    try {
      const allSnippets = await invoke<Snippet[]>("get_snippets");
      const isDuplicate = allSnippets.some(s => s.keyword === keyword.trim() && s.uuid !== snippet?.uuid);
      if (isDuplicate) {
        if (!window.confirm(`Apakah kamu yakin? Keyword '${keyword.trim()}' sudah pernah digunakan untuk snippet lainnya.`)) {
          return;
        }
      }
    } catch (e) { console.error(e); }

    setSaving(true);
    try {
      if (snippet) {
        await invoke("update_snippet_cmd", {
          s: { ...snippet, keyword: keyword.trim(), name, snippet: text, description: desc, group_id: groupId, matching_mode: matchingMode, case_sensitivity: caseSensitivity, modified_at: new Date().toISOString() }
        });
      } else {
        await invoke("add_snippet", { keyword: keyword.trim(), snippetText: text, name, description: desc, groupId, aiGenerated: false });
      }
      onSave();
    } catch (e) { showToast(String(e), "error"); }
    finally { setSaving(false); }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" style={{ maxWidth: 600 }} onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{snippet ? "✏️ Edit Snippet" : "✨ New Snippet"}</h2>
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div className="input-group">
              <label className="input-label">Keyword (Trigger)</label>
              <input className="input" placeholder="e.g. //email" value={keyword} onChange={e => setKeyword(e.target.value)} style={{ fontFamily: "var(--font-mono)" }} />
            </div>
            <div className="input-group">
              <label className="input-label">Name</label>
              <input className="input" placeholder="e.g. Formal Email" value={name} onChange={e => setName(e.target.value)} />
            </div>
          </div>

          <div className="input-group">
            <label className="input-label">Snippet Text</label>
            <div className="variable-toolbar">
              <span style={{ fontSize: 11, color: "var(--text-tertiary)", marginRight: 6 }}>Insert:</span>
              {["#{clipboard}", "#{date}", "#{time}", "#{input:}", "#{combo:}", "#{ai:}"].map(v => (
                <button key={v} className="var-btn" onClick={() => insertVariable(v)} title={v}>{v.replace("#{", "").replace("}", "")}</button>
              ))}
            </div>
            <textarea className="textarea" placeholder="The text that replaces the keyword..." value={text} onChange={e => setText(e.target.value)} rows={5} style={{ fontFamily: "var(--font-mono)", fontSize: 13 }} />
          </div>

          <div className="input-group">
            <label className="input-label">Description</label>
            <input className="input" placeholder="Optional description" value={desc} onChange={e => setDesc(e.target.value)} />
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12 }}>
            <div className="input-group">
              <label className="input-label">Group</label>
              <select className="input" value={groupId || ""} onChange={e => setGroupId(e.target.value || null)}>
                <option value="">No Group</option>
                {groups.map(g => <option key={g.uuid} value={g.uuid}>{g.name}</option>)}
              </select>
            </div>
            <div className="input-group">
              <label className="input-label">Matching Mode</label>
              <select className="input" value={matchingMode} onChange={e => setMatchingMode(e.target.value as any)}>
                <option value="Strict">Strict</option>
                <option value="Loose">Loose</option>
              </select>
            </div>
            <div className="input-group">
              <label className="input-label">Case Sensitivity</label>
              <select className="input" value={caseSensitivity} onChange={e => setCaseSensitivity(e.target.value as any)}>
                <option value="CaseSensitive">Case Sensitive</option>
                <option value="CaseInsensitive">Case Insensitive</option>
              </select>
            </div>
          </div>
        </div>
        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onClose}>Cancel</button>
          <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
            {saving ? <span className="spinner" /> : null}
            {snippet ? "Save Changes" : "Create Snippet"}
          </button>
        </div>
      </div>
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
                Select your Beeftext JSON file (usually <code style={{ color: "var(--accent-secondary)", fontSize: 12 }}>comboList.json</code> from <code style={{ color: "var(--accent-secondary)", fontSize: 12 }}>%AppData%\Beeftext\</code>)
              </p>
              <label className="file-upload-label">
                <input type="file" accept=".json" onChange={handleFile} style={{ display: "none" }} />
                <div className="file-upload-area">
                  {importing ? <span className="spinner" /> : "📂"}
                  <span>{importing ? "Importing..." : "Click to select JSON file"}</span>
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
                  <ul style={{ marginTop: 4 }}>{result.errors.slice(0, 5).map((e, i) => <li key={i}>{e}</li>)}</ul>
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
  const [messages, setMessages] = useState<{ role: string; content: string }[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<[string, string][]>("get_chat_history_cmd").then(history => {
      setMessages(history.map(([role, content]) => ({ role, content })));
    }).catch(() => {});
  }, []);

  useEffect(() => { messagesEndRef.current?.scrollIntoView({ behavior: "smooth" }); }, [messages]);

  const sendMessage = async () => {
    if (!input.trim() || loading) return;
    const userMsg = input.trim();
    setInput("");
    setMessages(prev => [...prev, { role: "user", content: userMsg }]);
    setLoading(true);
    try {
      const response = await invoke<string>("chat_with_ai", { message: userMsg });
      setMessages(prev => [...prev, { role: "assistant", content: response }]);
    } catch (e) {
      showToast(String(e), "error");
      setMessages(prev => [...prev, { role: "assistant", content: `❌ Error: ${e}` }]);
    } finally { setLoading(false); }
  };

  const handleClear = async () => {
    try { await invoke("clear_chat"); setMessages([]); showToast("Chat cleared"); }
    catch (e) { showToast(String(e), "error"); }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendMessage(); }
  };

  return (
    <div className="chat-container">
      <div className="content-header">
        <h1>🤖 AI Chat</h1>
        <button className="btn btn-secondary btn-sm" onClick={handleClear}>🗑 Clear Chat</button>
      </div>
      <div className="chat-messages">
        {messages.length === 0 && (
          <div className="empty-state">
            <div className="empty-state-icon">💬</div>
            <h3>Start a conversation</h3>
            <p>Ask the AI to create snippets, suggest keywords, or help organize your text library.<br />Try: "Buatkan snippet email izin sakit"</p>
          </div>
        )}
        {messages.map((msg, i) => (
          <div key={i} className={`chat-message ${msg.role}`}>
            <div className={`chat-avatar ${msg.role === "user" ? "user-avatar" : "ai-avatar"}`}>
              {msg.role === "user" ? "👤" : "🤖"}
            </div>
            <div className="chat-bubble">
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
        <div className="chat-input-wrapper">
          <textarea className="chat-input" placeholder="Ask AI to create a snippet..." value={input} onChange={e => setInput(e.target.value)} onKeyDown={handleKeyDown} rows={1} disabled={!ollamaOnline || loading} />
          <button className="chat-send-btn" onClick={sendMessage} disabled={!ollamaOnline || loading || !input.trim()}>➤</button>
        </div>
      </div>
    </div>
  );
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
          if (!window.confirm(`Apakah kamu yakin? Keyword '${generatedKeyword}' sudah pernah digunakan untuk snippet lainnya.`)) {
            return;
          }
        }

        await invoke("add_snippet", { keyword: generatedKeyword, snippetText: snippetJson.snippet || "", name: snippetJson.name || "", description: snippetJson.description || "", groupId: null, aiGenerated: true });
        showToast("✅ Snippet saved!");
      } catch (e) { showToast(String(e), "error"); }
    };
    return (
      <div>
        {textBefore && <p style={{ marginBottom: 10 }}>{textBefore}</p>}
        <div className="snippet-card-chat">
          <div className="snippet-card-chat-header">📋 Generated Snippet</div>
          {snippetJson.keyword && <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Keyword:</span><span className="keyword-badge">{snippetJson.keyword}</span></div>}
          {snippetJson.name && <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Name:</span><span className="snippet-card-chat-value">{snippetJson.name}</span></div>}
          {snippetJson.description && <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Desc:</span><span className="snippet-card-chat-value" style={{ color: "var(--text-secondary)", fontSize: 12 }}>{snippetJson.description}</span></div>}
          {snippetJson.group && <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Group:</span><span className="snippet-card-chat-value" style={{ color: "var(--text-secondary)", fontSize: 12 }}>{snippetJson.group}</span></div>}
          <div className="snippet-card-chat-field"><span className="snippet-card-chat-label">Snippet:</span><span className="snippet-card-chat-value" style={{ whiteSpace: "pre-wrap" }}>{snippetJson.snippet}</span></div>
          <div className="snippet-card-chat-actions"><button className="btn btn-primary btn-sm" onClick={handleSave}>✅ Save Snippet</button></div>
        </div>
        {textAfter && <p style={{ marginTop: 10 }}>{textAfter}</p>}
      </div>
    );
  }
  return <div style={{ whiteSpace: "pre-wrap" }}>{content}</div>;
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

function SearchPage({ showToast, ollamaOnline }: { showToast: (m: string, t?: "success" | "error") => void; ollamaOnline: boolean }) {
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
              <div key={s.uuid} className="card" style={{ padding: 16, display: "flex", alignItems: "center", gap: 16 }}>
                <div style={{ fontSize: 20, opacity: 0.5, width: 28, textAlign: "center" }}>{i + 1}</div>
                <div style={{ flex: 1 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 4 }}>
                    <span className="keyword-badge">{s.keyword}</span>
                    <span style={{ fontWeight: 600 }}>{s.name || "Untitled"}</span>
                    {s.ai_generated && <span className="ai-badge">🤖 AI</span>}
                  </div>
                  <div style={{ fontSize: 13, color: "var(--text-secondary)" }}>{s.snippet.substring(0, 120)}{s.snippet.length > 120 ? "..." : ""}</div>
                </div>
                {s.score !== undefined && (
                  <div style={{ textAlign: "right", minWidth: 60 }}>
                    <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>Relevance</div>
                    <div style={{ fontSize: 14, fontWeight: 700, color: s.score > 0.7 ? "var(--accent-success)" : "var(--text-secondary)" }}>{Math.round(s.score * 100)}%</div>
                  </div>
                )}
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

// ═══════════════════════════════════════════════════════════════════════════════
// Settings Page
// ═══════════════════════════════════════════════════════════════════════════════

function SettingsPage({ showToast, ollamaOnline }: { showToast: (m: string, t?: "success" | "error") => void; ollamaOnline: boolean }) {
  const [ollamaUrl, setOllamaUrl] = useState("http://localhost:11434");
  const [textModel, setTextModel] = useState("nemotron-3-super:cloud");
  const [embedModel, setEmbedModel] = useState("nomic-embed-text");
  const [language, setLanguage] = useState("both");
  const [models, setModels] = useState<{ name: string }[]>([]);
  const [hookActive, setHookActive] = useState(true);
  const [snippetCount, setSnippetCount] = useState(0);
  const [groupCount, setGroupCount] = useState(0);
  const [aiCount, setAiCount] = useState(0);
  const [embedCount, setEmbedCount] = useState(0);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [backingUp, setBackingUp] = useState(false);
  const [rebedding, setRebedding] = useState(false);
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);

  const loadData = useCallback(async () => {
    invoke<string | null>("get_preference", { key: "ollama_url" }).then(v => v && setOllamaUrl(v));
    invoke<string | null>("get_preference", { key: "text_model" }).then(v => v && setTextModel(v));
    invoke<string | null>("get_preference", { key: "embed_model" }).then(v => v && setEmbedModel(v));
    invoke<string | null>("get_preference", { key: "language" }).then(v => v && setLanguage(v));
    invoke<{ name: string }[]>("ollama_models").then(setModels).catch(() => {});
    invoke<boolean>("is_keyboard_hook_active").then(setHookActive).catch(() => {});
    invoke<boolean>("is_notifications_enabled").then(setNotificationsEnabled).catch(() => {});
    invoke<[number, number, number, number]>("get_snippet_stats").then(([t, _e, ai, emb]) => {
      setSnippetCount(t); setAiCount(ai); setEmbedCount(emb);
    }).catch(() => {});
    invoke<Group[]>("get_groups").then(g => setGroupCount(g.length)).catch(() => {});
    invoke<BackupInfo[]>("list_backups").then(setBackups).catch(() => {});
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const handleSave = async () => {
    try {
      await invoke("set_preference", { key: "ollama_url", value: ollamaUrl });
      await invoke("set_preference", { key: "text_model", value: textModel });
      await invoke("set_preference", { key: "embed_model", value: embedModel });
      await invoke("set_preference", { key: "language", value: language });
      showToast("Settings saved");
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleToggleHook = async () => {
    try {
      const newState = await invoke<boolean>("toggle_keyboard_hook", { enabled: !hookActive });
      setHookActive(newState);
      showToast(newState ? "✅ Text Expander enabled" : "⏸ Text Expander paused");
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleToggleNotifications = async () => {
    try {
      const newState = await invoke<boolean>("toggle_notifications", { enabled: !notificationsEnabled });
      setNotificationsEnabled(newState);
      showToast(newState ? "🔔 Notifications enabled" : "🔕 Notifications disabled");
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleCreateBackup = async () => {
    setBackingUp(true);
    try {
      const info = await invoke<BackupInfo>("create_backup");
      showToast(`✅ Backup created: ${info.snippet_count} snippets, ${info.group_count} groups`);
      loadData();
    } catch (e) { showToast(String(e), "error"); }
    finally { setBackingUp(false); }
  };

  const handleReEmbed = async () => {
    setRebedding(true);
    showToast("Starting background embedding... This may take a while.");
    try {
      const count = await invoke<number>("force_re_embed_all");
      showToast(`✅ Success! ${count} snippets embedded.`);
      loadData();
    } catch (e) { showToast(String(e), "error"); }
    finally { setRebedding(false); }
  };

  const handleRestoreBackup = async (filename: string) => {
    try {
      const [s, g] = await invoke<[number, number]>("restore_backup_cmd", { filename });
      showToast(`✅ Restored ${s} snippets and ${g} groups`);
      loadData();
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleDeleteBackup = async (filename: string) => {
    try {
      await invoke("delete_backup_cmd", { filename });
      showToast("Backup deleted");
      loadData();
    } catch (e) { showToast(String(e), "error"); }
  };

  const formatBytes = (bytes: number) => bytes < 1024 ? bytes + " B" : (bytes / 1024).toFixed(1) + " KB";

  return (
    <>
      <div className="content-header">
        <h1>⚙️ Settings</h1>
        <button className="btn btn-primary" onClick={handleSave}>💾 Save Settings</button>
      </div>
      <div className="content-body">
        {/* Overview Stats */}
        <div style={{ display: "flex", gap: 16, marginBottom: 32 }}>
          <div className="stat-card">
            <div className="stat-value">{snippetCount}</div>
            <div className="stat-label">Total Snippets</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{groupCount}</div>
            <div className="stat-label">Groups</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{embedCount}/{snippetCount}</div>
            <div className="stat-label">AI Embedded</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{aiCount}</div>
            <div className="stat-label">AI Generated</div>
          </div>
          <div className="stat-card">
            <div className="stat-value" style={{ fontSize: 18 }}>{hookActive ? "🟢 Active" : "⏸ Paused"}</div>
            <div className="stat-label">Text Expander</div>
          </div>
          <div className="stat-card">
            <div className="stat-value" style={{ fontSize: 18 }}>{ollamaOnline ? "🟢 Online" : "🔴 Offline"}</div>
            <div className="stat-label">Ollama AI</div>
          </div>
        </div>

        <div className="settings-section">
          <h3>⌨️ Text Expander Engine</h3>
          <div className="settings-row">
            <label>Background text expansion</label>
            <button className={`btn ${hookActive ? "btn-primary" : "btn-secondary"}`} onClick={handleToggleHook} style={{ minWidth: 120 }}>
              {hookActive ? "✅ Enabled" : "⏸ Disabled"}
            </button>
          </div>
          <div className="settings-row">
            <label>Desktop Notifications</label>
            <button className={`btn ${notificationsEnabled ? "btn-primary" : "btn-secondary"}`} onClick={handleToggleNotifications} style={{ minWidth: 120 }}>
              {notificationsEnabled ? "🔔 Enabled" : "🔕 Disabled"}
            </button>
          </div>
          <div style={{ fontSize: 12, color: "var(--text-tertiary)", padding: "8px 0" }}>
            When enabled, the text expander monitors your typing globally. When you type a snippet keyword (e.g. <code style={{ color: "var(--accent-secondary)" }}>//email</code>) and press space, the keyword is replaced with the snippet content. Notifications show a popup when a snippet is expanded.
          </div>
        </div>

        <div className="settings-section">
          <h3>🤖 AI Configuration</h3>
          <div className="settings-row"><label>Ollama URL</label><input className="input" value={ollamaUrl} onChange={e => setOllamaUrl(e.target.value)} style={{ maxWidth: 300 }} /></div>
          <div className="settings-row"><label>Text Generation Model</label><input className="input" value={textModel} onChange={e => setTextModel(e.target.value)} style={{ maxWidth: 300 }} /></div>
          <div className="settings-row"><label>Embedding Model</label><input className="input" value={embedModel} onChange={e => setEmbedModel(e.target.value)} style={{ maxWidth: 300 }} /></div>
          <div className="settings-row">
            <label>Status</label>
            <div className="status-indicator"><span className={`status-dot ${ollamaOnline ? "online" : "offline"}`} />{ollamaOnline ? "Connected" : "Disconnected"}</div>
          </div>
          <div className="settings-row">
            <label>Force Sync/Re-Embed</label>
            <button className="btn btn-secondary" onClick={handleReEmbed} disabled={rebedding || !ollamaOnline}>
              {rebedding ? <span className="spinner" /> : "🔄 Start Process"}
            </button>
          </div>
          {models.length > 0 && (
            <div className="settings-row" style={{ flexDirection: "column", alignItems: "flex-start", gap: 8 }}>
              <label>Available Models</label>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {models.map(m => <span key={m.name} className="keyword-badge" style={{ cursor: "pointer" }} onClick={() => setTextModel(m.name)}>{m.name}</span>)}
              </div>
            </div>
          )}
        </div>
        <div className="settings-section">
          <h3>🌐 Language / Bahasa</h3>
          <div className="settings-row">
            <label>Interface Language</label>
            <select className="input" value={language} onChange={e => setLanguage(e.target.value)} style={{ maxWidth: 300 }}>
              <option value="en">English</option>
              <option value="id">Bahasa Indonesia</option>
              <option value="both">Both / Keduanya</option>
            </select>
          </div>
        </div>
        <div className="settings-section">
          <h3>💾 Backup & Restore</h3>
          <div className="settings-row">
            <label>Create a backup of all snippets, groups, and settings</label>
            <button className="btn btn-primary" onClick={handleCreateBackup} disabled={backingUp}>
              {backingUp ? <span className="spinner" /> : "📦"} Create Backup
            </button>
          </div>
          {backups.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: 8 }}>
                Saved Backups ({backups.length})
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {backups.map(b => (
                  <div key={b.filename} className="card" style={{ padding: "10px 16px", display: "flex", alignItems: "center", gap: 12 }}>
                    <div style={{ flex: 1 }}>
                      <div style={{ fontSize: 13, fontWeight: 600 }}>{b.filename}</div>
                      <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
                        {new Date(b.created_at).toLocaleString()} · {b.snippet_count} snippets · {b.group_count} groups · {formatBytes(b.size_bytes)}
                      </div>
                    </div>
                    <button className="btn btn-sm btn-primary" onClick={() => handleRestoreBackup(b.filename)} style={{ fontSize: 11 }}>♻️ Restore</button>
                    <button className="btn btn-sm btn-danger btn-icon" onClick={() => handleDeleteBackup(b.filename)} title="Delete">🗑</button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
        <div className="settings-section">
          <h3>📋 Template Variables</h3>
          <div style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 2 }}>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{clipboard}"}</code> Current clipboard content<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{date}"}</code> Current date (YYYY-MM-DD)<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{time}"}</code> Current time (HH:MM:SS)<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{dateTime:format}"}</code> Custom date/time format<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{combo:keyword}"}</code> Insert another snippet<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{envVar:name}"}</code> Environment variable<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{ai:prompt}"}</code> Generate text via Ollama AI<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{upper:text}"}</code> Uppercase<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{lower:text}"}</code> Lowercase
          </div>
        </div>
        <div className="settings-section">
          <h3>🚀 Application Updates</h3>
          <div className="settings-row">
            <label>Check for new version</label>
            <UpdateChecker showToast={showToast} />
          </div>
        </div>

        <div className="settings-section">
          <h3>ℹ️ About</h3>
          <div className="settings-row"><label>Version</label><span style={{ color: "var(--text-secondary)" }}>BeefText AI v0.1.1</span></div>
          <div className="settings-row"><label>License</label><span style={{ color: "var(--text-secondary)" }}>MIT License</span></div>
          <div className="settings-row"><label>Inspired by</label><span style={{ color: "var(--text-secondary)" }}>Beeftext by Xavier Michelon</span></div>
        </div>
      </div>
    </>
  );
}

// ─── Update Checker ───────────────────────────────────────────────────────────

function UpdateChecker({ showToast }: { showToast: (m: string, t?: "success" | "error") => void }) {
  const [checking, setChecking] = useState(false);
  const [updateAvailable, setUpdateAvailable] = useState(false);

  const handleCheck = async () => {
    setChecking(true);
    try {
      const update = await check();
      if (update) {
        setUpdateAvailable(true);
        showToast("📦 An update is available!", "success");
        if (confirm(`New version ${update.version} is available. Install and restart?`)) {
          await update.downloadAndInstall();
          await relaunch();
        }
      } else {
        showToast("✅ Application is up to date");
      }
    } catch (e) {
      const errMsg = String(e);
      // Tauri updater throws this specific error if the latest.json endpoint returns a 404 (e.g. no release exists yet)
      if (errMsg.includes("JSON") || errMsg.includes("404")) {
        showToast("✅ You are already on the latest version");
      } else {
        showToast(`Update Check Failed: ${errMsg}`, "error");
      }
    } finally {
      setChecking(false);
    }
  };

  return (
    <button className={`btn ${updateAvailable ? "btn-primary" : "btn-secondary"}`} onClick={handleCheck} disabled={checking}>
      {checking ? <span className="spinner" /> : (updateAvailable ? "⬇️ Update Now" : "🔄 Check for Updates")}
    </button>
  );
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function downloadFile(filename: string, content: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
