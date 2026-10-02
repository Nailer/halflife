export type Theme = "dark" | "light";

const KEY = "halflife.theme";

export function initialTheme(): Theme {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === "dark" || saved === "light") return saved;
  } catch {
    /* storage unavailable; fall through to the system preference */
  }
  return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

export function applyTheme(t: Theme) {
  document.documentElement.setAttribute("data-theme", t);
  document.querySelector('meta[name="theme-color"]')
    ?.setAttribute("content", t === "light" ? "#f6f7f8" : "#0b0c0e");
  try {
    localStorage.setItem(KEY, t);
  } catch {
    /* preference simply will not persist */
  }
}
