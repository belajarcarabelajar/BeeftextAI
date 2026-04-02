import { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Snippet, Group } from "../types";

interface Props {
  snippet: Snippet | null;
  groups: Group[];
  onClose: () => void;
  onSave: () => void;
  showToast: (m: string, t?: "success" | "error") => void;
  t: (key: "confirmKeywordDuplicate", ...args: string[]) => string;
}

export default function SnippetEditor({ snippet, groups, onClose, onSave, showToast, t }: Props) {
  const [keyword, setKeyword] = useState(snippet?.keyword || "");
  const [name, setName] = useState(snippet?.name || "");
  const [text, setText] = useState(snippet?.snippet || "");
  const [desc, setDesc] = useState(snippet?.description || "");
  const [groupId, setGroupId] = useState<string | null>(snippet?.group_id || null);
  const [matchingMode, setMatchingMode] = useState<"Strict" | "Loose">(snippet?.matching_mode || "Strict");
  const [caseSensitivity, setCaseSensitivity] = useState<"CaseSensitive" | "CaseInsensitive">(snippet?.case_sensitivity || "CaseSensitive");
  const [contentType, setContentType] = useState<"Text" | "Image" | "Both">(snippet?.content_type || "Text");
  const [imageData, setImageData] = useState<string | null>(snippet?.image_data || null);
  const [imagePreview, setImagePreview] = useState<string | null>(snippet?.image_data || null);
  const [saving, setSaving] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const insertVariable = (v: string) => {
    const ta = textareaRef.current;
    if (!ta) {
      setText(prev => prev + v);
      return;
    }
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const before = text.slice(0, start);
    const after = text.slice(end);
    setText(before + v + after);
    requestAnimationFrame(() => {
      ta.focus();
      const newPos = start + v.length;
      ta.setSelectionRange(newPos, newPos);
    });
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

  const handleRemoveImage = () => {
    setImageData(null);
    setImagePreview(null);
  };

  const isTextRequired = contentType === "Text" || contentType === "Both";
  const isImageRequired = contentType === "Image" || contentType === "Both";

  const handleSave = async () => {
    if (!keyword.trim()) { showToast("Keyword is required", "error"); return; }
    if (isTextRequired && !text.trim()) { showToast("Snippet text is required for Text and Both types", "error"); return; }
    if (contentType !== "Text" && !imageData) { showToast("Please select an image", "error"); return; }

    try {
      const allSnippets = await invoke<Snippet[]>("get_snippets");
      const isDuplicate = allSnippets.some(s => s.keyword === keyword.trim() && s.uuid !== snippet?.uuid);
      if (isDuplicate) {
        if (!window.confirm(t("confirmKeywordDuplicate", keyword.trim()))) {
          return;
        }
      }
    } catch (e) { console.error(e); }

    setSaving(true);
    try {
      if (snippet) {
        await invoke("update_snippet_cmd", {
          s: { ...snippet, keyword: keyword.trim(), name, snippet: text, description: desc, group_id: groupId, matching_mode: matchingMode, case_sensitivity: caseSensitivity, modified_at: new Date().toISOString(), content_type: contentType },
          imageData,
        });
      } else {
        await invoke("add_snippet", { keyword: keyword.trim(), snippetText: text, name, description: desc, groupId, aiGenerated: false, imageData, contentType });
      }
      onSave();
    } catch (e) { showToast(String(e), "error"); }
    finally { setSaving(false); }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" style={{ maxWidth: 650 }} onClick={e => e.stopPropagation()}>
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
            <label className="input-label">Content Type</label>
            <div style={{ display: "flex", gap: 8 }}>
              {(["Text", "Image", "Both"] as const).map(ct => (
                <button key={ct}
                  className={`btn ${contentType === ct ? "btn-primary" : "btn-secondary"}`}
                  onClick={() => setContentType(ct)}
                  style={{ flex: 1, fontSize: 13 }}
                >
                  {ct === "Text" ? "📝 Text" : ct === "Image" ? "🖼️ Image" : "📝🖼️ Both"}
                </button>
              ))}
            </div>
          </div>

          {(contentType === "Text" || contentType === "Both") && (
            <div className="input-group">
              <label className="input-label">Snippet Text {contentType === "Both" && <span style={{ color: "var(--text-tertiary)" }}>(pasted first)</span>}</label>
              <div className="variable-toolbar">
                <span style={{ fontSize: 11, color: "var(--text-tertiary)", marginRight: 6 }}>Insert:</span>
                <select
                  className="var-select"
                  onChange={e => { if (e.target.value) { insertVariable(e.target.value); e.target.value = ""; } }}
                  value=""
                  title="Insert variable"
                >
                  <option value="">— Variable —</option>
                  {[
                    ["#{clipboard}", "#{clipboard}"],
                    ["#{date}", "#{date}"],
                    ["#{time}", "#{time}"],
                    ["#{dateTime:}", "#{dateTime:format}"],
                    ["#{date:}", "#{date:format}"],
                    ["#{time:}", "#{time:format}"],
                    ["#{envVar:}", "#{envVar:name}"],
                    ["#{cursor}", "#{cursor}"],
                    ["#{input:}", "#{input:description}"],
                    ["#{combo:}", "#{combo:keyword}"],
                    ["#{upper:}", "#{upper:text}"],
                    ["#{lower:}", "#{lower:text}"],
                    ["#{trim:}", "#{trim:text}"],
                    ["#{ai:}", "#{ai:prompt}"],
                    ["#{key:}", "#{key:keyname}"],
                    ["#{key::2}", "#{key:keyname:count}"],
                    ["#{shortcut:}", "#{shortcut:mod+key}"],
                    ["#{delay:}", "#{delay:ms}"],
                    ["#{powershell:}", "#{powershell:path}"],
                    ["#{powershell::10000}", "#{powershell:path:timeoutMs}"],
                  ].map(([val, label]) => (
                    <option key={val} value={val}>{label}</option>
                  ))}
                </select>
              </div>
              <textarea ref={textareaRef} className="textarea" placeholder="The text that replaces the keyword..." value={text} onChange={e => setText(e.target.value)} rows={5} style={{ fontFamily: "var(--font-mono)", fontSize: 13 }} />
            </div>
          )}

          {contentType === "Image" && (
            <div className="input-group">
              <label className="input-label">Image</label>
              <input className="input" type="file" accept="image/*" onChange={handleImageSelect} />
              {imagePreview && (
                <div style={{ marginTop: 8, position: "relative", display: "inline-block" }}>
                  <img src={imagePreview} alt="Preview" style={{ maxWidth: "100%", maxHeight: 200, borderRadius: 6, border: "1px solid var(--border)" }} />
                  <button className="btn btn-danger" onClick={handleRemoveImage} style={{ marginTop: 6 }}>Remove Image</button>
                </div>
              )}
            </div>
          )}

          {contentType === "Both" && (
            <div className="input-group">
              <label className="input-label">Image <span style={{ color: "var(--text-tertiary)" }}>(pasted after text, ~150ms delay)</span></label>
              <input className="input" type="file" accept="image/*" onChange={handleImageSelect} />
              {imagePreview && (
                <div style={{ marginTop: 8, position: "relative", display: "inline-block" }}>
                  <img src={imagePreview} alt="Preview" style={{ maxWidth: "100%", maxHeight: 200, borderRadius: 6, border: "1px solid var(--border)" }} />
                  <button className="btn btn-danger" onClick={handleRemoveImage} style={{ marginTop: 6 }}>Remove Image</button>
                </div>
              )}
            </div>
          )}

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
              <select className="input" value={matchingMode} onChange={e => setMatchingMode(e.target.value as "Strict" | "Loose")}>
                <option value="Strict">Strict</option>
                <option value="Loose">Loose</option>
              </select>
            </div>
            <div className="input-group">
              <label className="input-label">Case Sensitivity</label>
              <select className="input" value={caseSensitivity} onChange={e => setCaseSensitivity(e.target.value as "CaseSensitive" | "CaseInsensitive")}>
                <option value="CaseSensitive">Case Sensitive</option>
                <option value="CaseInsensitive">Case Insensitive</option>
              </select>
            </div>
          </div>
        </div>
        <div className="modal-footer">
          {snippet && (
            <button className="btn btn-danger" onClick={async () => {
              if (!window.confirm("Are you sure you want to delete this snippet?")) return;
              try {
                await invoke("delete_snippet_cmd", { uuid: snippet.uuid });
                onSave();
              } catch (e) { showToast(String(e), "error"); }
            }}>
              🗑 Delete
            </button>
          )}
          <div style={{ flex: 1 }} />
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
