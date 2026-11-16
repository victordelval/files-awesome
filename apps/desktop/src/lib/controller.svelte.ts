/**
 * Application controller: owns the editor view and document lifecycle, and
 * exposes reactive state for the components. Components stay presentational.
 */
import { openSearchPanel } from '@codemirror/search';
import type { EditorState, Text } from '@codemirror/state';
import { EditorView, type ViewUpdate } from '@codemirror/view';
import { tick } from 'svelte';
import {
  backend,
  type DocMeta,
  type DocPayload,
  errorMessage,
  isBackendError,
  type RecentEntry,
} from './backend';
import { countWords, extractOutline, type Heading, headingAt } from './editor/outline';
import { createEditorState, modeCompartment, modeExtension, type EditorHooks } from './editor/setup';
import { t } from './i18n';
import { loadPrefs, type Prefs, savePrefs } from './prefs';

export type Notice = 'modified' | 'conflict' | 'missing' | null;

export interface Command {
  id: string;
  label: string;
  keys?: string[];
  when?: () => boolean;
  run: () => unknown;
}

export interface ConfirmAction {
  id: string;
  label: string;
  primary?: boolean;
  danger?: boolean;
}

export interface ConfirmRequest {
  title: string;
  body: string;
  actions: ConfirmAction[];
  resolve: (id: string) => void;
}

export const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

/** "Mod+Shift+S" → "Ctrl+Shift+S" / "⌘⇧S". */
export function formatKeys(keys: string): string {
  if (isMac) {
    return keys
      .replace('Mod+', '⌘')
      .replace('Shift+', '⇧')
      .replace('Alt+', '⌥');
  }
  return keys.replace('Mod', 'Ctrl');
}

function comboOf(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (isMac ? e.metaKey : e.ctrlKey) parts.push('Mod');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');
  parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);
  return parts.join('+');
}

const TEXT_SIZES = [14, 15, 16, 17, 18, 19, 20, 22, 24];

class AppController {
  // Reactive state ----------------------------------------------------------
  doc = $state<DocMeta | null>(null);
  dirty = $state(false);
  notice = $state<Notice>(null);
  outline = $state<Heading[]>([]);
  activeHeading = $state(-1);
  words = $state(0);
  sourceMode = $state(false);
  focusMode = $state(false);
  paletteOpen = $state(false);
  confirm = $state<ConfirmRequest | null>(null);
  toast = $state<{ text: string; tone: 'info' | 'error'; id: number } | null>(null);
  recent = $state<RecentEntry[]>([]);
  prefs = $state<Prefs>(loadPrefs());
  ready = $state(false);

  displayName = $derived(this.doc ? this.doc.name || t.untitled : '');
  windowTitle = $derived(
    this.doc ? `${this.dirty ? '• ' : ''}${this.displayName} — ${t.app}` : t.app,
  );

  // Non-reactive internals --------------------------------------------------
  private view: EditorView | null = null;
  private pending: EditorState | null = null;
  private saved: Text | null = null;
  private overwriteNext = false;
  private analyseTimer = 0;
  private toastTimer = 0;
  private hooks: EditorHooks = {
    onUpdate: (u) => this.onEditorUpdate(u),
    onOpenLink: (href) => void this.openLink(href),
  };

  // Lifecycle ---------------------------------------------------------------
  async init() {
    this.applyPrefs();
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => this.applyPrefs());
    window.addEventListener('focus', () => void this.checkDisk());

    try {
      await backend.onDiskChanged((id) => {
        if (id === this.doc?.id) void this.checkDisk();
      });
      await backend.onOpenPaths((paths) => {
        if (paths[0]) void this.openPath(paths[0]);
      });
      await backend.onCloseRequested(() => this.confirmDiscard());

      await this.refreshRecent();
      const [first] = await backend.takeStartupPaths().catch(() => [] as string[]);
      if (first) await this.openPath(first);
    } catch (e) {
      this.showToast(errorMessage(e), 'error');
    } finally {
      // The window starts hidden (no white flash); always reveal it.
      this.ready = true;
      await tick();
      requestAnimationFrame(() => void backend.showWindow().catch(() => {}));
    }
  }

  mountEditor(parent: HTMLElement) {
    const state = this.pending ?? createEditorState('', this.sourceMode, this.hooks);
    this.pending = null;
    this.view = new EditorView({ state, parent });
    this.view.scrollDOM.addEventListener('scroll', () => this.scheduleActiveHeading(), { passive: true });
    this.analyse();
    this.view.focus();
  }

  unmountEditor() {
    this.view?.destroy();
    this.view = null;
  }

  // Documents ---------------------------------------------------------------
  async newDoc() {
    if (!(await this.confirmDiscard())) return;
    await this.run(async () => this.load(await backend.newDoc()));
  }

  async open() {
    if (!(await this.confirmDiscard())) return;
    await this.run(async () => {
      const payload = await backend.openDialog();
      if (payload) await this.load(payload);
    });
  }

  async openPath(path: string) {
    if (!(await this.confirmDiscard())) return;
    await this.run(async () => this.load(await backend.openPath(path)));
    await this.refreshRecent();
  }

  async closeDoc() {
    if (!this.doc || !(await this.confirmDiscard())) return;
    const id = this.doc.id;
    this.doc = null;
    this.dirty = false;
    this.notice = null;
    this.outline = [];
    this.focusMode = false;
    await backend.close(id).catch(() => {});
    await this.refreshRecent();
  }

  async save(): Promise<boolean> {
    if (!this.doc || !this.view) return false;
    if (!this.doc.path) return this.saveAs();
    const snapshot = this.view.state.doc;
    try {
      const result = await backend.save(this.doc.id, snapshot.toString(), this.overwriteNext);
      this.afterSave(result.meta, snapshot, result.historyWarning);
      return true;
    } catch (e) {
      if (isBackendError(e) && e.kind === 'conflict') this.notice = 'conflict';
      else this.showToast(errorMessage(e), 'error');
      return false;
    }
  }

  async saveAs(): Promise<boolean> {
    if (!this.doc || !this.view) return false;
    const snapshot = this.view.state.doc;
    try {
      const result = await backend.saveAs(this.doc.id, snapshot.toString());
      if (!result) return false;
      this.afterSave(result.meta, snapshot, result.historyWarning);
      await this.refreshRecent();
      return true;
    } catch (e) {
      this.showToast(errorMessage(e), 'error');
      return false;
    }
  }

  async reloadFromDisk(announce = false) {
    if (!this.doc?.path || !this.view) return;
    await this.run(async () => {
      const payload = await backend.reload(this.doc!.id);
      this.replaceText(payload.text);
      this.doc = stripText(payload);
      this.saved = this.view!.state.doc;
      this.dirty = false;
      this.notice = null;
      this.overwriteNext = false;
      if (announce) this.showToast(t.toast.reloaded);
    });
  }

  keepMine() {
    this.overwriteNext = true;
    this.notice = null;
  }

  async overwrite() {
    this.overwriteNext = true;
    this.notice = null;
    await this.save();
  }

  private async load(payload: DocPayload) {
    const previous = this.doc;
    if (previous && previous.id !== payload.id) await backend.close(previous.id).catch(() => {});

    const state = createEditorState(payload.text, this.sourceMode, this.hooks);
    this.saved = state.doc;
    this.dirty = false;
    this.notice = null;
    this.overwriteNext = false;
    this.doc = stripText(payload);

    if (this.view) {
      this.view.setState(state);
      this.view.scrollDOM.scrollTop = 0;
      this.analyse();
      this.view.focus();
    } else {
      this.pending = state; // Editor component mounts and picks it up
    }
  }

  private afterSave(meta: DocMeta, snapshot: Text, historyWarning: string | null) {
    this.doc = meta;
    this.saved = snapshot;
    this.dirty = !!this.view && !this.view.state.doc.eq(snapshot);
    this.overwriteNext = false;
    if (this.notice !== 'modified') this.notice = null;
    if (historyWarning) this.showToast(t.toast.historyWarning, 'error');
  }

  /** Replace the editor text with a minimal change, so the caret, scroll
   *  position and undo history survive (undo even reverts the reload). */
  private replaceText(next: string) {
    const view = this.view!;
    const current = view.state.doc.toString();
    if (current === next) return;
    let start = 0;
    const max = Math.min(current.length, next.length);
    while (start < max && current.charCodeAt(start) === next.charCodeAt(start)) start++;
    let end = 0;
    while (
      end < max - start &&
      current.charCodeAt(current.length - 1 - end) === next.charCodeAt(next.length - 1 - end)
    ) {
      end++;
    }
    view.dispatch({
      changes: { from: start, to: current.length - end, insert: next.slice(start, next.length - end) },
      userEvent: 'reload',
    });
  }

  async checkDisk() {
    if (!this.doc?.path) return;
    let state;
    try {
      state = await backend.diskState(this.doc.id);
    } catch {
      return;
    }
    if (state === 'unchanged') {
      if (this.notice === 'modified' || this.notice === 'missing') this.notice = null;
    } else if (state === 'missing') {
      this.notice = 'missing';
    } else if (!this.dirty) {
      await this.reloadFromDisk(true);
    } else if (!this.overwriteNext) {
      this.notice = 'modified';
    }
  }

  /** Resolves `true` when it is fine to drop the current document. */
  async confirmDiscard(): Promise<boolean> {
    if (!this.doc || !this.dirty) return true;
    const choice = await this.ask({
      title: t.confirm.unsavedTitle(this.displayName),
      body: t.confirm.unsavedBody,
      actions: [
        { id: 'cancel', label: t.confirm.cancel },
        { id: 'discard', label: t.confirm.discard, danger: true },
        { id: 'save', label: t.confirm.save, primary: true },
      ],
    });
    if (choice === 'save') return this.save();
    return choice === 'discard';
  }

  ask(request: Omit<ConfirmRequest, 'resolve'>): Promise<string> {
    return new Promise((resolve) => {
      this.confirm = {
        ...request,
        resolve: (id) => {
          this.confirm = null;
          resolve(id);
          this.view?.focus();
        },
      };
    });
  }

  async openLink(href: string) {
    if (/^(https?:|mailto:)/i.test(href)) {
      const choice = await this.ask({
        title: t.confirm.linkTitle,
        body: t.confirm.linkBody(href),
        actions: [
          { id: 'cancel', label: t.confirm.cancel },
          { id: 'open', label: t.confirm.open, primary: true },
        ],
      });
      if (choice === 'open') await this.run(() => backend.openExternal(href));
    } else if (/^[a-z][a-z0-9+.-]*:/i.test(href)) {
      this.showToast(t.toast.blockedLink, 'error');
    } else {
      this.showToast(t.toast.localLinks);
    }
  }

  // Editor bridge -----------------------------------------------------------
  private onEditorUpdate(u: ViewUpdate) {
    if (u.docChanged) {
      this.dirty = !this.saved || !u.state.doc.eq(this.saved);
      this.scheduleAnalyse();
    }
  }

  private scheduleAnalyse() {
    clearTimeout(this.analyseTimer);
    this.analyseTimer = window.setTimeout(() => this.analyse(), 180);
  }

  private analyse() {
    if (!this.view) return;
    const state = this.view.state;
    this.outline = extractOutline(state);
    this.words = countWords(state.doc.toString());
    this.updateActiveHeading();
  }

  private headingFrame = 0;
  private scheduleActiveHeading() {
    cancelAnimationFrame(this.headingFrame);
    this.headingFrame = requestAnimationFrame(() => this.updateActiveHeading());
  }

  private updateActiveHeading() {
    const view = this.view;
    if (!view || this.outline.length === 0) {
      this.activeHeading = -1;
      return;
    }
    const top = view.scrollDOM.getBoundingClientRect().top + 120;
    const block = view.lineBlockAtHeight(top - view.documentTop);
    this.activeHeading = headingAt(this.outline, block.from);
  }

  goTo(pos: number) {
    const view = this.view;
    if (!view) return;
    view.dispatch({
      selection: { anchor: pos },
      effects: EditorView.scrollIntoView(pos, { y: 'start', yMargin: 96 }),
    });
    view.focus();
  }

  // View modes & preferences ------------------------------------------------
  toggleSource() {
    this.sourceMode = !this.sourceMode;
    this.view?.dispatch({ effects: modeCompartment.reconfigure(modeExtension(this.sourceMode, this.hooks)) });
    this.view?.focus();
  }

  toggleFocus() {
    this.focusMode = !this.focusMode;
    this.view?.focus();
  }

  setPrefs(patch: Partial<Prefs>) {
    this.prefs = { ...this.prefs, ...patch };
    savePrefs(this.prefs);
    this.applyPrefs();
  }

  private applyPrefs() {
    const root = document.documentElement;
    const dark =
      this.prefs.theme === 'dark' ||
      (this.prefs.theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    root.dataset.theme = dark ? 'dark' : 'light';
    root.dataset.textFont = this.prefs.textFont;
    root.style.setProperty('--text-size', `${this.prefs.textSize}px`);
    this.view?.requestMeasure();
  }

  private zoom(step: number) {
    const i = TEXT_SIZES.indexOf(this.prefs.textSize);
    const next = TEXT_SIZES[Math.max(0, Math.min(TEXT_SIZES.length - 1, (i < 0 ? 3 : i) + step))];
    this.setPrefs({ textSize: next ?? 17 });
  }

  // Commands ----------------------------------------------------------------
  readonly commands: Command[] = [
    { id: 'new', label: t.cmd.new, keys: ['Mod+N'], run: () => this.newDoc() },
    { id: 'open', label: t.cmd.open, keys: ['Mod+O'], run: () => this.open() },
    { id: 'save', label: t.cmd.save, keys: ['Mod+S'], when: () => !!this.doc, run: () => this.save() },
    { id: 'saveAs', label: t.cmd.saveAs, keys: ['Mod+Shift+S'], when: () => !!this.doc, run: () => this.saveAs() },
    { id: 'reload', label: t.cmd.reload, when: () => !!this.doc?.path, run: () => this.reloadFromDisk(true) },
    { id: 'close', label: t.cmd.close, keys: ['Mod+W'], when: () => !!this.doc, run: () => this.closeDoc() },
    { id: 'find', label: t.cmd.find, keys: ['Mod+F'], when: () => !!this.view, run: () => this.view && openSearchPanel(this.view) },
    { id: 'source', label: t.cmd.toggleSource, keys: ['Mod+/', 'Mod+Shift+/'], when: () => !!this.doc, run: () => this.toggleSource() },
    { id: 'outline', label: t.cmd.toggleOutline, keys: ['Mod+Shift+O'], run: () => this.setPrefs({ outline: !this.prefs.outline }) },
    { id: 'focus', label: t.cmd.focus, keys: ['Mod+.'], when: () => !!this.doc, run: () => this.toggleFocus() },
    {
      id: 'theme',
      label: t.cmd.theme,
      run: () => {
        const order = ['system', 'light', 'dark'] as const;
        this.setPrefs({ theme: order[(order.indexOf(this.prefs.theme) + 1) % order.length] });
      },
    },
    { id: 'font', label: t.cmd.font, run: () => this.setPrefs({ textFont: this.prefs.textFont === 'sans' ? 'serif' : 'sans' }) },
    { id: 'zoomIn', label: t.cmd.zoomIn, keys: ['Mod+=', 'Mod++', 'Mod+Shift++'], run: () => this.zoom(1) },
    { id: 'zoomOut', label: t.cmd.zoomOut, keys: ['Mod+-'], run: () => this.zoom(-1) },
    { id: 'zoomReset', label: t.cmd.zoomReset, keys: ['Mod+0'], run: () => this.setPrefs({ textSize: 17 }) },
    { id: 'palette', label: t.cmd.palette, keys: ['Mod+P', 'Mod+Shift+P'], run: () => (this.paletteOpen = true) },
  ];

  handleKey(e: KeyboardEvent) {
    if (this.confirm) return; // the dialog owns the keyboard
    if (e.key === 'Escape' && this.focusMode && !this.paletteOpen) {
      this.focusMode = false;
      return;
    }
    const combo = comboOf(e);
    const command = this.commands.find((c) => c.keys?.includes(combo));
    if (command && (command.when?.() ?? true)) {
      e.preventDefault();
      this.paletteOpen = false;
      void command.run();
    }
  }

  // Helpers -----------------------------------------------------------------
  showToast(text: string, tone: 'info' | 'error' = 'info') {
    clearTimeout(this.toastTimer);
    this.toast = { text, tone, id: Date.now() };
    this.toastTimer = window.setTimeout(() => (this.toast = null), tone === 'error' ? 6000 : 2200);
  }

  private async run(action: () => Promise<unknown>) {
    try {
      await action();
    } catch (e) {
      this.showToast(errorMessage(e), 'error');
    }
  }

  private async refreshRecent() {
    this.recent = await backend.recent().catch(() => [] as RecentEntry[]);
  }

  focusEditor() {
    this.view?.focus();
  }
}

function stripText({ text: _text, ...meta }: DocPayload): DocMeta {
  return meta;
}

export const app = new AppController();
