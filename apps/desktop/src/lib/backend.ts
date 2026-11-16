/**
 * The only module that talks to the native side. Everything else in the UI
 * depends on the `Backend` interface, so the app also runs in a plain browser
 * (`pnpm dev:web`) with an in-memory demo backend — handy for design work.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import sample from './sample.md?raw';

export type LineEnding = 'lf' | 'crlf';
export interface TextFormat {
  lineEnding: LineEnding;
  bom: boolean;
  mixedLineEndings: boolean;
}
export interface DocMeta {
  id: number;
  path: string | null;
  name: string;
  dir: string | null;
  format: TextFormat;
}
export interface DocPayload extends DocMeta {
  text: string;
}
export interface SavePayload {
  meta: DocMeta;
  wrote: boolean;
  historyWarning: string | null;
}
export type DiskState = 'unchanged' | 'modified' | 'missing';
export interface RecentEntry {
  path: string;
  name: string;
  dir: string;
}
export interface BackendError {
  kind: string;
  message: string;
}

export function isBackendError(e: unknown): e is BackendError {
  return typeof e === 'object' && e !== null && 'kind' in e && 'message' in e;
}

export function errorMessage(e: unknown): string {
  if (isBackendError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}

type Unlisten = () => void;

export interface Backend {
  readonly native: boolean;
  takeStartupPaths(): Promise<string[]>;
  newDoc(): Promise<DocPayload>;
  openDialog(): Promise<DocPayload | null>;
  openPath(path: string): Promise<DocPayload>;
  save(id: number, text: string, overwrite: boolean): Promise<SavePayload>;
  saveAs(id: number, text: string): Promise<SavePayload | null>;
  reload(id: number): Promise<DocPayload>;
  close(id: number): Promise<void>;
  diskState(id: number): Promise<DiskState>;
  recent(): Promise<RecentEntry[]>;
  openExternal(url: string): Promise<void>;
  onDiskChanged(cb: (id: number) => void): Promise<Unlisten>;
  onOpenPaths(cb: (paths: string[]) => void): Promise<Unlisten>;
  /** `cb` resolves to `true` to let the window close. */
  onCloseRequested(cb: () => Promise<boolean>): Promise<Unlisten>;
  setTitle(title: string): Promise<void>;
  showWindow(): Promise<void>;
}

const tauriBackend: Backend = {
  native: true,
  takeStartupPaths: () => invoke('app_take_startup_paths'),
  newDoc: () => invoke('doc_new'),
  openDialog: () => invoke('doc_open_dialog'),
  openPath: (path) => invoke('doc_open_path', { path }),
  save: (id, text, overwrite) => invoke('doc_save', { id, text, overwrite }),
  saveAs: (id, text) => invoke('doc_save_as', { id, text }),
  reload: (id) => invoke('doc_reload', { id }),
  close: (id) => invoke('doc_close', { id }),
  diskState: (id) => invoke('doc_disk_state', { id }),
  recent: () => invoke('recent_list'),
  openExternal: (url) => invoke('open_external', { url }),
  onDiskChanged: (cb) => listen<number>('doc:disk-changed', (e) => cb(e.payload)),
  onOpenPaths: (cb) => listen<string[]>('app:open-paths', (e) => cb(e.payload)),
  onCloseRequested: (cb) =>
    getCurrentWindow().onCloseRequested(async (event) => {
      if (!(await cb())) event.preventDefault();
    }),
  setTitle: (title) => getCurrentWindow().setTitle(title),
  showWindow: () => getCurrentWindow().show(),
};

/** In-memory stand-in used when running outside Tauri. */
function demoBackend(): Backend {
  const docs = new Map<number, DocPayload>();
  let next = 1;
  const format: TextFormat = { lineEnding: 'lf', bom: false, mixedLineEndings: false };
  const make = (name: string, text: string, path: string | null): DocPayload => {
    const doc = { id: next++, name, path, dir: path ? '~/Documentos/demo' : null, format, text };
    docs.set(doc.id, doc);
    return doc;
  };
  const get = (id: number) => {
    const doc = docs.get(id);
    if (!doc) throw { kind: 'unknownDocument', message: `no document ${id}` } satisfies BackendError;
    return doc;
  };
  const noop = async () => () => {};
  return {
    native: false,
    takeStartupPaths: async () => ['demo://Manual.md'],
    newDoc: async () => make('', '', null),
    openDialog: async () => make('Manual.md', sample, 'demo://Manual.md'),
    openPath: async () => make('Manual.md', sample, 'demo://Manual.md'),
    save: async (id, text) => {
      const doc = get(id);
      doc.text = text;
      return { meta: doc, wrote: true, historyWarning: null };
    },
    saveAs: async (id, text) => {
      const doc = get(id);
      Object.assign(doc, { text, name: doc.name || 'Sin título.md', path: 'demo://' + (doc.name || 'Sin título.md'), dir: '~/Documentos/demo' });
      return { meta: doc, wrote: true, historyWarning: null };
    },
    reload: async (id) => get(id),
    close: async (id) => void docs.delete(id),
    diskState: async () => 'unchanged',
    recent: async () => [
      { path: 'demo://Manual.md', name: 'Manual.md', dir: '~/Documentos/demo' },
      { path: 'demo://Notas de obra.md', name: 'Notas de obra.md', dir: '~/Proyectos/casa-patio' },
      { path: 'demo://Memoria descriptiva.md', name: 'Memoria descriptiva.md', dir: '~/Proyectos/casa-patio/entregas' },
    ],
    openExternal: async (url) => void window.open(url, '_blank', 'noopener'),
    onDiskChanged: noop,
    onOpenPaths: noop,
    onCloseRequested: noop,
    setTitle: async (title) => void (document.title = title),
    showWindow: async () => {},
  };
}

export const backend: Backend =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window ? tauriBackend : demoBackend();
