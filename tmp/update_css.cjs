const fs = require('fs');

let css = fs.readFileSync('src/index.css', 'utf8');

const newRoot = `:root {
  /* ── Color Palette ── */
  --bg-primary: #0a0a0a;
  --bg-secondary: #000000;
  --bg-tertiary: #161616;
  --bg-card: #121212;
  --bg-card-hover: #1c1c1c;
  --bg-glass: #121212;
  --bg-input: #111111;

  --accent-primary: #ffc107;
  --accent-secondary: #fc5c1e;
  --accent-gradient: linear-gradient(135deg, #ffc107 0%, #fc5c1e 100%);
  --accent-glow: rgba(255, 193, 7, 0.3);
  --accent-success: #10b981;
  --accent-warning: #ffbf00;
  --accent-error: #e52b2b;
  --accent-info: #3b82f6;

  --text-primary: #ffffff;
  --text-secondary: #cccccc;
  --text-tertiary: #888888;
  --text-accent: #ffc107;

  --border-subtle: #262626;
  --border-medium: #333333;
  --border-accent: rgba(255, 193, 7, 0.4);

  /* ── Typography ── */
  --font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;

  /* ── Spacing ── */
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 16px;
  --radius-xl: 24px;

  /* ── Shadows ── */
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.4);
  --shadow-lg: 0 8px 30px rgba(0, 0, 0, 0.5);
  --shadow-glow: 0 0 20px var(--accent-glow);

  /* ── Transitions (Disabled for Speed) ── */
  --ease-out: none;
  --ease-spring: none;
}`;

css = css.replace(/:root\s*{[\s\S]*?--ease-spring:[^}]+}/, newRoot);

css = css.replace(/transition:\s*[^;{}]+;/g, '');
css = css.replace(/animation:\s*[^;{}]+;/g, '');
css = css.replace(/backdrop-filter:\s*[^;{}]+;/g, '');
css = css.replace(/-webkit-backdrop-filter:\s*[^;{}]+;/g, '');

fs.writeFileSync('src/index.css', css);
