/** Per-user UI preferences, kept in localStorage (never document data). */
export type ThemePref = 'system' | 'light' | 'dark';
export type TextFont = 'sans' | 'serif';

export interface Prefs {
  theme: ThemePref;
  textFont: TextFont;
  textSize: number;
  outline: boolean;
}

const KEY = 'fa.prefs.v1';
const DEFAULTS: Prefs = { theme: 'system', textFont: 'sans', textSize: 17, outline: true };

export function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? { ...DEFAULTS, ...(JSON.parse(raw) as Partial<Prefs>) } : { ...DEFAULTS };
  } catch {
    return { ...DEFAULTS };
  }
}

export function savePrefs(prefs: Prefs): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {
    /* storage unavailable: preferences just won't persist */
  }
}
