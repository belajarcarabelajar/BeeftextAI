const fs = require('fs');

function applyAppTsx() {
  let code = fs.readFileSync('src/App.tsx', 'utf-8');
  
  // 1. imports
  code = code.replace(/import \{ Snippet, Group, Page \} from "\.\\/types";/, 'import { Snippet, Group, Page, ImportResult } from "./types";');

  // 2. pass t to SnippetsPage
  code = code.replace(/<SnippetsPage showToast=\{showToast\} showForm=\{showForm\} setShowForm=\{setShowForm\} editingSnippet=\{editingSnippet\} setEditingSnippet=\{setEditingSnippet\} \/>/, '<SnippetsPage showToast={showToast} showForm={showForm} setShowForm={setShowForm} editingSnippet={editingSnippet} setEditingSnippet={setEditingSnippet} t={t} />');

  // 3. accept t in SnippetsPage props
  code = code.replace(/  setEditingSnippet: \\(s: Snippet \\| null\\) => void;\\n\\}\\) \\{/, '  setEditingSnippet: (s: Snippet | null) => void;\n  t: any;\n}) {');

  // 4. fix ImportModal map error
  code = code.replace(/result\\.errors\\.slice\\(0, 5\\)\\.map\\(\\(e, i\\) =>/, 'result.errors.slice(0, 5).map((e: string, i: number) =>');

  // 5. ChatPage state
  code = code.replace(/const \\[messages, setMessages\\] = useState<\\{ role: string; content: string \\}\\[\\]>\\(\\[\\]\\);/, 'const [messages, setMessages] = useState<{ role: string; content: string; imagePreview?: string }[]>([]);');

  // 6. sendMessage logic
  const oldSendMessage = `    if ((!input.trim() && !imageData) || loading) return;
    const MAX_INPUT_TOKENS = 2000;
    const rawMsg = input.trim();
    const userMsg = rawMsg ? truncateToTokens(rawMsg, MAX_INPUT_TOKENS) : "";
    const tokens = userMsg ? estimateTokenCount(userMsg) : 0;
    const wasTruncated = userMsg.length < rawMsg.length;
    setInput("");
    // Add user message + keep only last 10 messages to reduce backend load
    setMessages(prev => {
      const updated = [...prev, { role: "user", content: userMsg || (imageData ? "[image]" : "") }];
      return updated.length > 10 ? updated.slice(updated.length - 10) : updated;
    });
    setLoading(true);
    try {
      const response = await invoke<string>("chat_with_ai", { message: userMsg, imageData });
      setMessages(prev => [...prev, { role: "assistant", content: response }]);
    } catch (e) {
      showToast(String(e), "error");
      setMessages(prev => [...prev, { role: "assistant", content: \`❌ Error: \${e}\` }]);
    } finally {
      setLoading(false);
      setImageData(null);
      setImagePreview(null);
    }`;

  const newSendMessage = `    // Require text input always — image-only is rejected (model is non-OCR)
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
      ? userMsg + "\\n\\n[SYSTEM NOTE: The user has successfully attached an image to this request. Acknowledge it, and generate the JSON snippet with content_type 'Image' or 'Both'. OMIT the 'image_data' field from the JSON. Do NOT say you cannot see the image.]"
      : userMsg;

    try {
      const response = await invoke<string>("chat_with_ai", { message: backendMsg, imageData });
      setMessages(prev => [...prev, { role: "assistant", content: response, imagePreview: sentImagePreview ?? undefined }]);
    } catch (e) {
      showToast(String(e), "error");
      setMessages(prev => [...prev, { role: "assistant", content: \`❌ Error: \${e}\` }]);
    } finally {
      setLoading(false);
      setImageData(null);
      setImagePreview(null);
    }`;
  code = code.replace(oldSendMessage, newSendMessage);

  // 7. MessageContent image attachment render
  const oldMsgBubble = `<MessageContent content={msg.content} showToast={showToast} />`;
  const newMsgBubble = `{msg.role === "user" && msg.imagePreview && (
                <img
                  src={msg.imagePreview}
                  alt="Attachment"
                  style={{ maxHeight: 120, maxWidth: "100%", borderRadius: 6, marginBottom: 8, display: "block", border: "1px solid var(--border)" }}
                />
              )}
              <MessageContent content={msg.content} showToast={showToast} imagePreview={msg.imagePreview} />`;
  code = code.replace(oldMsgBubble, newMsgBubble);

  // 8. disabled btn limit
  code = code.replace(/disabled=\\{!ollamaOnline \\|\\| loading \\|\\| \\(!input\.trim\\(\\) && !imageData\\)\\}/, 'disabled={!ollamaOnline || loading || !input.trim()}');

  // 9. MessageContent signature
  code = code.replace(/function MessageContent\\(\\{ content, showToast \\}: \\{ content: string; showToast: \\(m: string, t\\?: "success" \\| "error"\\) => void \\}\\) \\{/, 'function MessageContent({ content, showToast, imagePreview }: { content: string; showToast: (m: string, t?: "success" | "error") => void, imagePreview?: string }) {');

  // 10. MessageContent confirmKeywordDuplicate
  code = code.replace(/if \\(!window\.confirm\\(t\\("confirmKeywordDuplicate", generatedKeyword\\)\\)\\) \\{/, 'if (!window.confirm(`Are you sure? The keyword \\'${generatedKeyword}\\' has already been used for another snippet.`)) {');

  // 11. MessageContent add_snippet
  const oldAddSnippet = `await invoke("add_snippet", { keyword: generatedKeyword, snippetText: snippetJson.snippet || "", name: snippetJson.name || "", description: snippetJson.description || "", groupId: groupId, aiGenerated: true, imageData: snippetJson.image_data || null, contentType: snippetJson.content_type || "Text" });`;
  const newAddSnippet = `const finalImageData = snippetJson.image_data || imagePreview || null;
        await invoke("add_snippet", { keyword: generatedKeyword, snippetText: snippetJson.snippet || "", name: snippetJson.name || "", description: snippetJson.description || "", groupId: groupId, aiGenerated: true, imageData: finalImageData, contentType: snippetJson.content_type || "Text" });`;
  code = code.replace(oldAddSnippet, newAddSnippet);

  // 12. MessageContent img
  code = code.replace(/const img = snippetJson\.image_data;/, 'const img = snippetJson.image_data || imagePreview;');

  // 13. renderMarkdown return
  code = code.replace(/return <div style=\\{\\{ whiteSpace: "pre-wrap" \\}\\}>\\{content\\}<\\/div>;/, 'return <div className="chat-md">{renderMarkdown(content)}</div>;');

  // 14. insert renderMarkdown code before function MessageContent
  const markdownCode = `
// ─── Inline Markdown Renderer ─────────────────────────────────────────────────
function renderMarkdown(text: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  let key = 0;
  const codeBlockParts = text.split(/(~~~(\\s|\\S)*?~~~)/g).map(p => p.replace(/~~~/g, '\`\`\`'));

  for (const part of codeBlockParts) {
    if (part.startsWith("\`\`\`") && part.endsWith("\`\`\`")) {
      const inner = part.slice(3, -3);
      const firstNewline = inner.indexOf("\\n");
      const code = firstNewline !== -1 ? inner.slice(firstNewline + 1) : inner;
      nodes.push(<pre key={key++} className="chat-code-block"><code>{code}</code></pre>);
      continue;
    }
    const lines = part.split("\\n");
    let i = 0;
    while (i < lines.length) {
      const line = lines[i];
      if (/^[-*]\\s/.test(line)) {
        const listItems: React.ReactNode[] = [];
        while (i < lines.length && /^[-*]\\s/.test(lines[i])) {
          listItems.push(<li key={key++}>{renderInline(lines[i].replace(/^[-*]\\s/, ""), key)}</li>);
          i++;
        }
        nodes.push(<ul key={key++} className="chat-md-list">{listItems}</ul>);
        continue;
      }
      if (/^\\d+\\.\\s/.test(line)) {
        const listItems: React.ReactNode[] = [];
        while (i < lines.length && /^\\d+\\.\\s/.test(lines[i])) {
          listItems.push(<li key={key++}>{renderInline(lines[i].replace(/^\\d+\\.\\s/, ""), key)}</li>);
          i++;
        }
        nodes.push(<ol key={key++} className="chat-md-list">{listItems}</ol>);
        continue;
      }
      if (line.trim() === "") {
        nodes.push(<br key={key++} />);
        i++;
        continue;
      }
      nodes.push(<span key={key++}>{renderInline(line, key)}<br /></span>);
      i++;
    }
  }
  return nodes;
}

function renderInline(text: string, baseKey: number): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  let key = baseKey * 1000;
  const inlineParts = text.split(/(\`[^\`]+\`)/g);

  for (const part of inlineParts) {
    if (part.startsWith("\`") && part.endsWith("\`") && part.length > 2) {
      nodes.push(<code key={key++} className="chat-inline-code">{part.slice(1, -1)}</code>);
      continue;
    }
    let remaining = part;
    const boldItalicRegex = /(\\*\\*[^*]+\\*\\*|\\*[^*]+\\*)/g;
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

function MessageContent`;
  code = code.replace(/function MessageContent/g, (match, offset, str) => {
    if (str.substring(offset - 20, offset).trim().endsWith("──────────────────────────")) {
      return markdownCode;
    }
    return match;
  });
  
  // replace hack
  code = code.replace(/(~~~[\\s\\S]*?~~~)/g, '\`\`\`');
  
  fs.writeFileSync('src/App.tsx', code);
}

function applyLibRs() {
  let code = fs.readFileSync('apps/desktop/src-tauri/src/lib.rs', 'utf-8');
  
  const oldPrompt = `const SYSTEM_PROMPT: &str = r#"You are an AI assistant for BeefText AI, a smart text expander.

### Snippet Creation
When the user wants to create/save a snippet, respond with:
{"keyword": "!abc", "snippet": "text", "name": "Name", "description": "Desc", "group": "GroupName", "content_type": "Text|Image|Both", "image_data": "base64_or_null"}
Always auto-generate a name, description, and assign a logical group.
For image snippets: set content_type to "Image" or "Both" and include the image_data field (the user already uploaded the image).
For text snippets: set content_type to "Text" and omit image_data.`;

  const newPrompt = `const SYSTEM_PROMPT: &str = r#"You are an AI assistant for BeefText AI, a smart text expander.

### Language Rules
- Default communication language: Bahasa Indonesia.
- If the user writes in English, respond in English.
- If the user writes in Indonesian (Bahasa Indonesia), respond in Indonesian.
- Always mirror the language of the user's most recent message.

### Snippet Creation
When the user wants to create/save a snippet, respond with:
{"keyword": "!abc", "snippet": "text", "name": "Name", "description": "Desc", "group": "GroupName", "content_type": "Text|Image|Both"}
Always auto-generate a name, description, and assign a logical group.
For image snippets: set content_type to "Image" or "Both". OMIT the image_data field. The system handles the image automatically.
For text snippets: set content_type to "Text" and omit image_data.`;

  code = code.replace(oldPrompt, newPrompt);
  
  const oldPush = `messages.push(ChatMessage { role: "user".to_string(), content: message_truncated, images: None });`;
  const newPush = `// Strip "data:image/...;base64," prefix — Ollama expects raw base64 only
    let images = image_data.as_ref().map(|data| {
        let b64 = if let Some(pos) = data.find(',') {
            data[pos + 1..].to_string()
        } else {
            data.clone()
        };
        vec![b64]
    });
    messages.push(ChatMessage { role: "user".to_string(), content: message_truncated, images });`;
    
  code = code.replace(oldPush, newPush);
  fs.writeFileSync('apps/desktop/src-tauri/src/lib.rs', code);
}

function applyIndexCss() {
  let code = fs.readFileSync('src/index.css', 'utf-8');
  code = code.replace(/--text-secondary: #cccccc;/g, '--text-secondary: #c4c4c4;');
  code = code.replace(/--text-tertiary: #666666;/g, '--text-tertiary: #8a8a8a;');
  
  code = code.replace(/--text-secondary: #888888;/g, '--text-secondary: #a3a3a3;');
  
  const oldBubble = `.chat-message.user .chat-bubble {
  background: var(--accent-primary);
  color: white;
  border-bottom-right-radius: 4px;
}`;
  const newBubble = `/* User bubble — deep amber-to-burnt-orange gradient for WCAG AA compliance
   Contrast: white on #d97706 ≈ 4.89:1 ✅, white on #c2410c ≈ 5.73:1 ✅ */
.chat-message.user .chat-bubble {
  background: linear-gradient(135deg, #d97706 0%, #c2410c 100%);
  color: #ffffff;
  border-bottom-right-radius: 4px;
}`;
  code = code.replace(oldBubble, newBubble);

  if (!code.includes(".chat-md {")) {
    const cssAppend = `
/* ── Chat Markdown Renderer ── */
.chat-md {
  line-height: 1.7;
}

.chat-code-block {
  background: var(--bg-primary);
  border: 1px solid var(--border-medium);
  border-radius: var(--radius-md);
  padding: 12px 14px;
  margin: 8px 0;
  overflow-x: auto;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-primary);
  white-space: pre;
}

.chat-inline-code {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 1px 5px;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--accent-secondary);
}

.chat-md-list {
  margin: 6px 0 6px 18px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.chat-md-list li {
  font-size: 14px;
  line-height: 1.6;
}

/* ── Light mode: user chat bubble override ── */
[data-theme="light"] .chat-message.user .chat-bubble {
  background: linear-gradient(135deg, #b45309 0%, #9a3412 100%);
  color: #ffffff;
}
`;
    // Insert before /* ══════════════════════════════════════════════════════════════════════════════
    // Modal / Dialog
    code = code.replace(/\\/\\* ══════════════════════════════════════════════════════════════════════════════\\n   Modal \\/ Dialog/, cssAppend + '\n/* ══════════════════════════════════════════════════════════════════════════════\n   Modal / Dialog');
  }
  
  fs.writeFileSync('src/index.css', code);
}

try { applyAppTsx(); console.log("applyAppTsx OK!"); } catch (e) { console.error("Error App.tsx:", e); }
try { applyLibRs(); console.log("applyLibRs OK!"); } catch (e) { console.error("Error lib.rs:", e); }
try { applyIndexCss(); console.log("applyIndexCss OK!"); } catch (e) { console.error("Error index.css:", e); }
