import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import { Group } from "../types";
import { Language } from "../i18n";
import { formatBytes } from "../utils";

interface Props {
  showToast: (m: string, t?: "success" | "error") => void;
  ollamaOnline: boolean;
  onLanguageChange: (lang: Language) => void;
}

export default function SettingsPanel({ showToast, ollamaOnline, onLanguageChange }: Props) {
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
  const [backups, setBackups] = useState<import("../types").BackupInfo[]>([]);
  const [backingUp, setBackingUp] = useState(false);
  const [rebedding, setRebedding] = useState(false);
  const [embedProgress, setEmbedProgress] = useState<{ current: number; total: number; percentage: number } | null>(null);
  const [embedState, setEmbedState] = useState<'idle' | 'running' | 'paused'>('idle');
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);

  // Model auto-detection: categorize as embedding or text generation
  const isEmbeddingModel = (name: string): boolean => {
    const embedPatterns = ['embed', 'nomic', 'qn3', 'e5', 'bge', 'gte', 'cohere', 'jina', 'mxbai', 'sentence'];
    const lower = name.toLowerCase();
    return embedPatterns.some(p => lower.includes(p));
  };
  const embeddingModels = models.filter(m => isEmbeddingModel(m.name));
  const textModels = models.filter(m => !isEmbeddingModel(m.name));

  const handlePauseEmbed = async () => {
    try {
      await invoke("pause_embedding");
      setEmbedState('paused');
      showToast("Embedding paused");
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleResumeEmbed = async () => {
    try {
      await invoke("resume_embedding");
      setEmbedState('running');
      showToast("Embedding resumed");
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleStopEmbed = async () => {
    try {
      await invoke("stop_embedding");
      setEmbedState('idle');
      showToast("Embedding stopped");
    } catch (e) { showToast(String(e), "error"); }
  };

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
    invoke<import("../types").BackupInfo[]>("list_backups").then(setBackups).catch(() => {});

    const unlisten = listen<{ current: number; total: number; percentage: number }>("embed_progress", (event) => {
      setEmbedProgress(event.payload);
    });
    return () => { unlisten.then(f => f()); };
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
      const info = await invoke<import("../types").BackupInfo>("create_backup");
      showToast(`✅ Backup created: ${info.snippet_count} snippets, ${info.group_count} groups`);
      loadData();
    } catch (e) { showToast(String(e), "error"); }
    finally { setBackingUp(false); }
  };

  const handleReEmbed = async (resume: boolean = false) => {
    setRebedding(true);
    setEmbedState('running');
    setEmbedProgress(null);
    showToast(resume ? "Resuming embedding process..." : "Starting fresh embedding process...");
    try {
      const result = await invoke<{ successful: number; failed: number; failures: { uuid: string; name: string; reason: string }[] }>("force_re_embed_all", { resume });
      if (result.failed > 0) {
        showToast(`⚠️ ${result.successful} embedded, ${result.failed} failed. Check console for details.`, "error");
      } else {
        showToast(`✅ Success! ${result.successful} snippets embedded.`);
      }
      loadData();
    } catch (e) { showToast(String(e), "error"); }
    finally { setRebedding(false); setEmbedState('idle'); }
  };

  const handleRestoreBackup = async (filename: string) => {
    if (!window.confirm(`Restore backup "${filename}"? Current data will be replaced.`)) return;
    try {
      await invoke("restore_backup", { filename });
      showToast("✅ Backup restored");
      loadData();
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleDeleteBackup = async (filename: string) => {
    if (!window.confirm(`Delete backup "${filename}"?`)) return;
    try {
      await invoke("delete_backup", { filename });
      loadData();
    } catch (e) { showToast(String(e), "error"); }
  };

  const handleCheckUpdate = async () => {
    try {
      const update = await check();
      if (update && confirm(`New version ${update.version} is available. Install and restart?`)) {
        await update.downloadAndInstall();
        await relaunch();
      } else {
        showToast("✅ You are already on the latest version");
      }
    } catch (e) {
      const errMsg = String(e);
      showToast(`Update Check Failed: ${errMsg}`, "error");
    }
  };

  return (
    <>
      <div className="content-header">
        <h1>⚙️ Settings</h1>
        <button className="btn btn-primary" onClick={handleSave}>💾 Save Settings</button>
      </div>
      <div className="content-body">
        <div style={{ display: "flex", gap: 16, marginBottom: 32 }}>
          <div className="stat-card"><div className="stat-value">{snippetCount}</div><div className="stat-label">Total Snippets</div></div>
          <div className="stat-card"><div className="stat-value">{groupCount}</div><div className="stat-label">Groups</div></div>
          <div className="stat-card"><div className="stat-value">{embedCount}/{snippetCount}</div><div className="stat-label">AI Embedded</div></div>
          <div className="stat-card"><div className="stat-value">{aiCount}</div><div className="stat-label">AI Generated</div></div>
          <div className="stat-card"><div className="stat-value" style={{ fontSize: 18 }}>{hookActive ? "🟢 Active" : "⏸ Paused"}</div><div className="stat-label">Text Expander</div></div>
          <div className="stat-card"><div className="stat-value" style={{ fontSize: 18 }}>{ollamaOnline ? "🟢 Online" : "🔴 Offline"}</div><div className="stat-label">Ollama AI</div></div>
        </div>

        <div className="settings-section">
          <h3>⌨️ Text Expander Engine</h3>
          <div className="settings-row"><label>Background text expansion</label><button className={`btn ${hookActive ? "btn-primary" : "btn-secondary"}`} onClick={handleToggleHook} style={{ minWidth: 120 }}>{hookActive ? "✅ Enabled" : "⏸ Disabled"}</button></div>
          <div className="settings-row"><label>Desktop Notifications</label><button className={`btn ${notificationsEnabled ? "btn-primary" : "btn-secondary"}`} onClick={handleToggleNotifications} style={{ minWidth: 120 }}>{notificationsEnabled ? "🔔 Enabled" : "🔕 Disabled"}</button></div>
          <div style={{ fontSize: 12, color: "var(--text-tertiary)", padding: "8px 0" }}>When enabled, the text expander monitors your typing globally. When you type a snippet keyword (e.g. <code style={{ color: "var(--accent-secondary)" }}>//email</code>) and press space, the keyword is replaced with the snippet content. Notifications show a popup when a snippet is expanded.</div>
        </div>

        <div className="settings-section">
          <h3>🤖 AI Configuration</h3>
          <div className="settings-row"><label>Ollama URL</label><input className="input" value={ollamaUrl} onChange={e => setOllamaUrl(e.target.value)} style={{ maxWidth: 300 }} /></div>
          <div className="settings-row"><label>Text Generation Model</label><input className="input" value={textModel} onChange={e => setTextModel(e.target.value)} style={{ maxWidth: 300 }} /></div>
          <div className="settings-row"><label>Embedding Model</label><input className="input" value={embedModel} onChange={e => setEmbedModel(e.target.value)} style={{ maxWidth: 300 }} /></div>
          <div className="settings-row"><label>Status</label><div className="status-indicator"><span className={`status-dot ${ollamaOnline ? "online" : "offline"}`} />{ollamaOnline ? "Connected" : "Disconnected"}</div></div>
          <div className="settings-row" style={{ flexDirection: "column", alignItems: "flex-start", gap: 12 }}>
            <label>Semantic Search Sync</label>
            <div style={{ display: "flex", gap: 10, width: "100%" }}>
              {embedState === 'idle' && (
                <>
                  <button className="btn btn-secondary" onClick={() => handleReEmbed(true)} disabled={rebedding || !ollamaOnline} style={{ flex: 1 }}>{rebedding ? <span className="spinner" /> : "▶ Start Embedding"}</button>
                  <button className="btn btn-danger btn-outline" onClick={() => { if(confirm("This will clear existing embeddings and start from scratch. Proceed?")) handleReEmbed(false) }} disabled={rebedding || !ollamaOnline} style={{ flex: 1 }}>{rebedding ? <span className="spinner" /> : "🔄 Force All"}</button>
                </>
              )}
              {embedState === 'running' && (
                <>
                  <button className="btn btn-secondary" onClick={handlePauseEmbed} disabled={!ollamaOnline} style={{ flex: 1 }}>⏸ Pause</button>
                  <button className="btn btn-danger btn-outline" onClick={handleStopEmbed} disabled={!ollamaOnline} style={{ flex: 1 }}>⏹ Stop</button>
                </>
              )}
              {embedState === 'paused' && (
                <>
                  <button className="btn btn-secondary" onClick={handleResumeEmbed} disabled={!ollamaOnline} style={{ flex: 1 }}>▶ Resume</button>
                  <button className="btn btn-danger btn-outline" onClick={handleStopEmbed} disabled={!ollamaOnline} style={{ flex: 1 }}>⏹ Stop</button>
                </>
              )}
            </div>
            {embedProgress && (
              <div className="progress-container" style={{ width: "100%", marginTop: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6, fontSize: 12 }}>
                  <span style={{ color: "var(--text-secondary)" }}>Processing {embedProgress.current} of {embedProgress.total}</span>
                  <span style={{ fontWeight: 600, color: "var(--accent-primary)" }}>{Math.round(embedProgress.percentage)}%</span>
                </div>
                <div className="progress-bar-bg"><div className="progress-bar-fill" style={{ width: `${embedProgress.percentage}%` }}></div></div>
              </div>
            )}
            <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>Embedding is required for OmniSearch (semantic). Click a model below to auto-populate the correct field.</div>
          </div>
          {models.length > 0 && (
            <div style={{ display: "flex", gap: 24, marginTop: 8 }}>
              {embeddingModels.length > 0 && (
                <div style={{ flex: 1 }}>
                  <label style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, display: "block" }}>Embedding Models</label>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                    {embeddingModels.map(m => <span key={m.name} className="keyword-badge" style={{ cursor: "pointer" }} onClick={() => setEmbedModel(m.name)}>{m.name}</span>)}
                  </div>
                </div>
              )}
              {textModels.length > 0 && (
                <div style={{ flex: 1 }}>
                  <label style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, display: "block" }}>Text Generation Models</label>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                    {textModels.map(m => <span key={m.name} className="keyword-badge" style={{ cursor: "pointer" }} onClick={() => setTextModel(m.name)}>{m.name}</span>)}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        <div className="settings-section">
          <h3>🌐 Language / Bahasa</h3>
          <div className="settings-row">
            <label>Interface Language</label>
            <select className="input" value={language} onChange={e => { const v = e.target.value as Language; setLanguage(v); onLanguageChange(v); invoke("set_preference", { key: "language", value: v }); }} style={{ maxWidth: 300 }}>
              <option value="en">English</option>
              <option value="id">Bahasa Indonesia</option>
              <option value="both">Both / Keduanya</option>
            </select>
          </div>
        </div>

        <div className="settings-section">
          <h3>💾 Backup & Restore</h3>
          <div className="settings-row"><label>Create a backup of all snippets, groups, and settings</label><button className="btn btn-primary" onClick={handleCreateBackup} disabled={backingUp}>{backingUp ? <span className="spinner" /> : "📦"} Create Backup</button></div>
          {backups.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: 8 }}>Saved Backups ({backups.length})</div>
              {backups.map(b => (
                <div key={b.filename} className="card" style={{ padding: "10px 16px", display: "flex", alignItems: "center", gap: 12 }}>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 13, fontWeight: 600 }}>{b.filename}</div>
                    <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{new Date(b.created_at).toLocaleString()} · {b.snippet_count} snippets · {b.group_count} groups · {formatBytes(b.size_bytes)}</div>
                  </div>
                  <button className="btn btn-sm btn-primary" onClick={() => handleRestoreBackup(b.filename)} style={{ fontSize: 11 }}>♻️ Restore</button>
                  <button className="btn btn-sm btn-danger btn-icon" onClick={() => handleDeleteBackup(b.filename)} title="Delete">🗑</button>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="settings-section">
          <h3>📋 Template Variables</h3>
          <div style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 2 }}>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{clipboard}"}</code> Current clipboard content<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{date}"}</code> Current date (YYYY-MM-DD)<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{time}"}</code> Current time (HH:MM:SS)<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{dateTime:format}"}</code> Custom date/time (e.g. yyyy-MM-dd HH:mm:ss)<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{dateTime:+1d:format}"}</code> Date with offset (e.g. +1d-2h)<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{date:format}"}</code> Custom date format<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{time:format}"}</code> Custom time format<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{cursor}"}</code> Cursor position after paste<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{input:description}"}</code> Interactive text input dialog<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{combo:keyword}"}</code> Insert another snippet<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{envVar:name}"}</code> Environment variable<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{ai:prompt}"}</code> Generate text via Ollama AI<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{upper:text}"}</code> Uppercase<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{lower:text}"}</code> Lowercase<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{trim:text}"}</code> Trim whitespace<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{key:keyname}"}</code> Simulate key press (e.g. tab, enter, up)<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{key:keyname:count}"}</code> Repeat key N times<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{shortcut:mod+key}"}</code> Simulate shortcut (e.g. Ctrl+Shift+J)<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{delay:ms}"}</code> Pause during expansion (milliseconds)<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{powershell:path}"}</code> Execute PowerShell script<br/>
            <code className="keyword-badge" style={{ marginRight: 8 }}>{"#{powershell:path:timeoutMs}"}</code> Script with timeout (0=indefinite)
          </div>
        </div>

        <div className="settings-section">
          <h3>🚀 Application Updates</h3>
          <div className="settings-row">
            <label>Check for new version</label>
            <button className="btn btn-secondary" onClick={handleCheckUpdate}>🔄 Check for Updates</button>
          </div>
        </div>

        <div className="settings-section">
          <h3>ℹ️ About</h3>
          <div className="settings-row"><label>Version</label><span style={{ color: "var(--text-secondary)" }}>BeefText AI</span></div>
          <div className="settings-row"><label>License</label><span style={{ color: "var(--text-secondary)" }}>MIT License</span></div>
          <div className="settings-row"><label>Inspired by</label><span style={{ color: "var(--text-secondary)" }}>Beeftext by Xavier Michelon</span></div>
        </div>
      </div>
    </>
  );
}
