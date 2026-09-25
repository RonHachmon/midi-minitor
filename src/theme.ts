import type { ThemeDto } from "./bindings";

/**
 * Painting the window in the chosen theme.
 *
 * # Why the attribute goes on `<html>`
 *
 * Tailwind emits the design tokens on `:root`, so the override block in
 * `index.css` has to match that element or an ancestor of it — and `<html>` is
 * the only one there is. It is also the element `color-scheme` must sit on for
 * the scrollbars and the canvas behind the document to follow.
 *
 * # Why the value written is the wire id
 *
 * `"default"` and `"raver"` are what `ThemeDto` serialises to and what the CSS
 * selects on. Passing the value straight through means there is no translation
 * table to keep in step with either side, and a value from a later version that
 * this stylesheet does not know simply fails to match and leaves the default
 * palette standing.
 */
const THEME_ATTRIBUTE = "data-theme";

/**
 * Where the last applied theme is cached for the next launch's first paint.
 *
 * # Why a cache exists when Rust holds the setting
 *
 * The stored settings are only reachable over IPC, which resolves after the
 * document has painted — so a user on `Raver` would see the light window for a
 * frame on every launch. The inline script in `index.html` reads this key
 * synchronously before the stylesheet is parsed, which removes that frame.
 *
 * It is a cache and not a source of truth: whatever the core answers overwrites
 * it milliseconds later, so the two can only disagree for one frame, and only
 * when the settings document was changed by something other than this app.
 *
 * **The same string is spelt out literally in `index.html`.** That script cannot
 * import from here — it has to be inline and synchronous to beat first paint, so
 * it runs before any module does. Changing either spelling means changing both.
 * The duplication would disappear if the window were built in Rust with
 * `WebviewWindowBuilder` and an `initialization_script`, but the window is
 * declared in `tauri.conf.json` and is created before `setup()` runs, so taking
 * that route would mean moving the window's geometry out of configuration and
 * into code — too much to pay for one string.
 */
const THEME_CACHE_KEY = "midi-monitor.theme";

/** Paints the window in this theme, and remembers it for the next first paint. */
export function applyTheme(theme: ThemeDto): void {
  document.documentElement.setAttribute(THEME_ATTRIBUTE, theme);
  try {
    localStorage.setItem(THEME_CACHE_KEY, theme);
  } catch {
    // A webview with storage unavailable costs one frame of the wrong palette
    // on the next launch. Not worth failing a theme change over.
  }
}
