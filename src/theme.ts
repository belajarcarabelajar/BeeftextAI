// ─── Theme Constants ───────────────────────────────────────────────────────────

export type Theme = "dark" | "light";
const STORAGE_KEY = "beeftext-theme";

// ─── Core Functions ───────────────────────────────────────────────────────────

/** Get the effective theme: localStorage → system → dark (default) */
export function getPreferredTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY) as Theme | null;
  if (stored === "dark" || stored === "light") return stored;

  if (window.matchMedia?.("(prefers-color-scheme: light)").matches) {
    return "light";
  }

  return "dark";
}

/** Apply theme attribute to <html> and persist to localStorage */
export function setTheme(theme: Theme): void {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem(STORAGE_KEY, theme);
}

/** Toggle from current to the other theme */
export function toggleTheme(): Theme {
  const next: Theme = getPreferredTheme() === "dark" ? "light" : "dark";
  setTheme(next);
  return next;
}

/** Get stored theme from localStorage (null if never set) */
export function getStoredTheme(): Theme | null {
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored === "dark" || stored === "light" ? stored : null;
}

/** Initialize theme on app startup — call once at app entry */
export function initTheme(): void {
  setTheme(getPreferredTheme());
}
