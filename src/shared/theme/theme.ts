export type ThemeMode = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export interface ThemeSettings {
  mode: ThemeMode;
  highContrast: boolean;
  customBrandColor: string | null;
}

const MIRROR_KEY = "ygocmg.themeSettings";
const LEGACY_MODE_KEY = "ygocmg.themeMode";
const BRAND_STYLE_ID = "ygocmg-custom-brand";
const VALID_MODES: ReadonlyArray<ThemeMode> = ["system", "light", "dark"];
const HEX_COLOR = /^#[0-9a-fA-F]{6}$/;

function systemPrefersDark(): boolean {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return false;
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function resolveTheme(mode: ThemeMode, prefersDark = systemPrefersDark()): ResolvedTheme {
  if (mode === "light") return "light";
  if (mode === "dark") return "dark";
  return prefersDark ? "dark" : "light";
}

function relativeLuminance(hex: string): number {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const channel = (c: number) => (c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function pickTextOnBrand(hex: string): string {
  return relativeLuminance(hex) > 0.45 ? "#081414" : "#ffffff";
}

function applyBrandOverride(color: string | null): void {
  const existing = document.getElementById(BRAND_STYLE_ID);
  if (!color) {
    existing?.remove();
    return;
  }
  const textOnBrand = pickTextOnBrand(color);
  const css = `:root, :root[data-theme="dark"] {
  --brand: ${color};
  --brand-hover: color-mix(in srgb, ${color} 86%, white);
  --brand-soft: color-mix(in srgb, ${color} 18%, transparent);
  --brand-soft-strong: color-mix(in srgb, ${color} 28%, transparent);
  --line-brand: color-mix(in srgb, ${color} 32%, transparent);
  --line-brand-strong: color-mix(in srgb, ${color} 50%, transparent);
  --focus-ring: color-mix(in srgb, ${color} 32%, transparent);
  --text-on-brand: ${textOnBrand};
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --brand-hover: color-mix(in srgb, ${color} 78%, white);
  }
}`;
  const node = existing ?? Object.assign(document.createElement("style"), { id: BRAND_STYLE_ID });
  node.textContent = css;
  if (!existing) document.head.appendChild(node);
}

export function applyThemeSettings(settings: ThemeSettings): ResolvedTheme {
  const root = document.documentElement;
  const resolved = resolveTheme(settings.mode);
  if (settings.mode === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", resolved);
  }
  root.style.colorScheme = resolved;
  if (settings.highContrast) {
    root.setAttribute("data-high-contrast", "true");
  } else {
    root.removeAttribute("data-high-contrast");
  }
  applyBrandOverride(settings.customBrandColor && HEX_COLOR.test(settings.customBrandColor) ? settings.customBrandColor : null);
  return resolved;
}

export function applyThemeToDocument(mode: ThemeMode): ResolvedTheme {
  return applyThemeSettings({ mode, highContrast: false, customBrandColor: null });
}

export function readThemeMirror(): ThemeSettings | null {
  try {
    const raw = window.localStorage.getItem(MIRROR_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (
        parsed &&
        typeof parsed === "object" &&
        (VALID_MODES as ReadonlyArray<string>).includes(parsed.mode)
      ) {
        return {
          mode: parsed.mode as ThemeMode,
          highContrast: Boolean(parsed.highContrast),
          customBrandColor:
            typeof parsed.customBrandColor === "string" && HEX_COLOR.test(parsed.customBrandColor)
              ? parsed.customBrandColor
              : null,
        };
      }
    }
    const legacy = window.localStorage.getItem(LEGACY_MODE_KEY);
    if (legacy && (VALID_MODES as ReadonlyArray<string>).includes(legacy)) {
      return { mode: legacy as ThemeMode, highContrast: false, customBrandColor: null };
    }
  } catch {
    // storage unavailable
  }
  return null;
}

export function writeThemeMirror(settings: ThemeSettings): void {
  try {
    window.localStorage.setItem(MIRROR_KEY, JSON.stringify(settings));
  } catch {
    // storage unavailable
  }
}

export function watchSystemTheme(listener: (prefersDark: boolean) => void): () => void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return () => {};
  }
  const mql = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = (event: MediaQueryListEvent) => listener(event.matches);
  mql.addEventListener("change", handler);
  return () => mql.removeEventListener("change", handler);
}

export function bootstrapThemeFromMirror(): void {
  const mirror = readThemeMirror() ?? { mode: "system" as ThemeMode, highContrast: false, customBrandColor: null };
  applyThemeSettings(mirror);
}
