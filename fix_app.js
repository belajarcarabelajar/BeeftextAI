const fs = require('fs');
let code = fs.readFileSync('src/App.tsx', 'utf-8');

// 1. imports
code = code.replace(/import \{ Snippet, Group, Page \} from "\.\/types";/, 'import { Snippet, Group, Page, ImportResult } from "./types";');

// 2. pass t to SnippetsPage
code = code.replace(/<SnippetsPage showToast=\{showToast\} showForm=\{showForm\} setShowForm=\{setShowForm\} editingSnippet=\{editingSnippet\} setEditingSnippet=\{setEditingSnippet\} \/>/, '<SnippetsPage showToast={showToast} showForm={showForm} setShowForm={setShowForm} editingSnippet={editingSnippet} setEditingSnippet={setEditingSnippet} t={t as any} />');

// 3. accept t in SnippetsPage props
code = code.replace(/  setEditingSnippet: \(s: Snippet \| null\) => void;\n\}\) \{/, '  setEditingSnippet: (s: Snippet | null) => void;\n  t: any;\n}) {');

// 4. fix ImportModal map error
code = code.replace(/result\.errors\.slice\(0, 5\)\.map\(\(e, i\) =>/, 'result.errors.slice(0, 5).map((e: string, i: number) =>');

// 5. ChatPage state
code = code.replace(/const \[messages, setMessages\] = useState<\{ role: string; content: string \}\[\]>\(\[\]\);/, 'const [messages, setMessages] = useState<{ role: string; content: string; imagePreview?: string }[]>([]);');

// 6. sendMessage logic
code = code.replace(/    if \(\(!input\.trim\(\) && !imageData\) \|\| loading\) return;\n    const MAX_INPUT_TOKENS = 2000;\n    const rawMsg = input\.trim\(\);\n    const userMsg = rawMsg \? truncateToTokens\(rawMsg, MAX_INPUT_TOKENS\) : "";\n    const tokens = userMsg \? estimateTokenCount\(userMsg\) : 0;\n    const wasTruncated = userMsg\.length < rawMsg\.length;\n    setInput\(""\);\n    \/\/ Add user message \+ keep only last 10 messages to reduce backend load\n    setMessages\(prev => \{\n      const updated = \[\.\.\.prev, \{ role: "user", content: userMsg \|\| \(imageData \? "\[image\]" : ""\) \}\];\n      return updated\.length > 10 \? updated\.slice\(updated\.length - 10\) : updated;\n    \}\);\n    setLoading\(true\);\n    try \{\n      const response = await invoke<string>\("chat_with_ai", \{ message: userMsg, imageData \}\);\n      setMessages\(prev => \[\.\.\.prev, \{ role: "assistant", content: response \}\]\);\n/, `    // Require text input always — image-only is rejected (model is non-OCR)
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
    setInput("");
    const sentImagePreview = imagePreview; // capture before clearing
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
`);

// 7. MessageContent image
code = code.replace(/<MessageContent content=\{msg\.content\} showToast=\{showToast\} \/>/g, `{msg.role === "user" && msg.imagePreview && (
                <img
                  src={msg.imagePreview}
                  alt="Attachment"
                  style={{ maxHeight: 120, maxWidth: "100%", borderRadius: 6, marginBottom: 8, display: "block", border: "1px solid var(--border)" }}
                />
              )}
              <MessageContent content={msg.content} showToast={showToast} imagePreview={msg.imagePreview} />`);

// 8. disabled btn limit
code = code.replace(/disabled=\{!ollamaOnline \|\| loading \|\| \(!input\.trim\(\) && !imageData\)\}/, 'disabled={!ollamaOnline || loading || !input.trim()}');

// 9. MessageContent signature
code = code.replace(/function MessageContent\(\{ content, showToast \}: \{ content: string; showToast: \(m: string, t\?: "success" \| "error"\) => void \}\) \{/, 'function MessageContent({ content, showToast, imagePreview }: { content: string; showToast: (m: string, t?: "success" | "error") => void, imagePreview?: string }) {');

// 10. MessageContent confirmKeywordDuplicate
code = code.replace(/if \(!window\.confirm\(t\("confirmKeywordDuplicate", generatedKeyword\)\)\) \{/, 'if (!window.confirm(`Are you sure? The keyword \\'${generatedKeyword}\\' has already been used for another snippet.`)) {');

// 11. MessageContent add_snippet
code = code.replace(/await invoke\("add_snippet", \{ keyword: generatedKeyword, snippetText: snippetJson\.snippet \|\| "", name: snippetJson\.name \|\| "", description: snippetJson\.description \|\| "", groupId: groupId, aiGenerated: true, imageData: snippetJson\.image_data \|\| null, contentType: snippetJson\.content_type \|\| "Text" \}\);/, `const finalImageData = snippetJson.image_data || imagePreview || null;
        await invoke("add_snippet", { keyword: generatedKeyword, snippetText: snippetJson.snippet || "", name: snippetJson.name || "", description: snippetJson.description || "", groupId: groupId, aiGenerated: true, imageData: finalImageData, contentType: snippetJson.content_type || "Text" });`);

// 12. MessageContent img
code = code.replace(/const img = snippetJson\.image_data;/, 'const img = snippetJson.image_data || imagePreview;');

// 13. renderMarkdown return
code = code.replace(/return <div style=\{\{ whiteSpace: "pre-wrap" \}\}>\{content\}<\/div>;/, 'return <div className="chat-md">{renderMarkdown(content)}</div>;');

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

// Fix regex escaping for codeBlockParts
code = code.replace(/(~~~(\\s|\\S)*?~~~)/g, '```');

fs.writeFileSync('src/App.tsx', code);
console.log('done');
