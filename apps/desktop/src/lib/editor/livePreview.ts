/**
 * Live preview: Markdown rendered in place, Obsidian-style.
 *
 * The document is never transformed. Everything here is a *decoration* over
 * the original text: markup is hidden on lines the caret is not on and shows
 * up again as soon as you move into them. What you see is a view; what gets
 * saved is exactly what you typed.
 */
import { syntaxTree } from '@codemirror/language';
import type { EditorState, Extension, Range } from '@codemirror/state';
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from '@codemirror/view';
import type { SyntaxNode, SyntaxNodeRef } from '@lezer/common';

export interface LivePreviewOptions {
  /** Called on Ctrl/Cmd+click on a link. */
  onOpenLink: (href: string) => void;
}

// ── Decoration atoms ─────────────────────────────────────────────────────────

const hiddenMark = Decoration.replace({});
const cache = new Map<string, Decoration>();
const line = (cls: string) => {
  const key = 'l:' + cls;
  let d = cache.get(key);
  if (!d) cache.set(key, (d = Decoration.line({ class: cls })));
  return d;
};
const mark = (cls: string) => {
  const key = 'm:' + cls;
  let d = cache.get(key);
  if (!d) cache.set(key, (d = Decoration.mark({ class: cls })));
  return d;
};

class BulletWidget extends WidgetType {
  override eq() {
    return true;
  }
  toDOM() {
    const el = document.createElement('span');
    el.className = 'cm-bullet';
    el.setAttribute('aria-hidden', 'true');
    return el;
  }
}

class CheckboxWidget extends WidgetType {
  constructor(
    readonly checked: boolean,
    readonly pos: number,
  ) {
    super();
  }
  override eq(other: CheckboxWidget) {
    return other.checked === this.checked && other.pos === this.pos;
  }
  toDOM() {
    const el = document.createElement('span');
    el.className = 'cm-task-box' + (this.checked ? ' is-checked' : '');
    el.setAttribute('role', 'checkbox');
    el.setAttribute('aria-checked', String(this.checked));
    el.dataset.pos = String(this.pos);
    return el;
  }
  override ignoreEvent() {
    return false;
  }
}

class RuleWidget extends WidgetType {
  override eq() {
    return true;
  }
  toDOM() {
    const el = document.createElement('span');
    el.className = 'cm-rule';
    return el;
  }
}

class ImageWidget extends WidgetType {
  constructor(
    readonly alt: string,
    readonly src: string,
  ) {
    super();
  }
  override eq(other: ImageWidget) {
    return other.alt === this.alt && other.src === this.src;
  }
  toDOM() {
    // Images are not loaded yet: remote ones will stay blocked by default
    // (privacy) and local ones need a scoped asset protocol. For now, a chip.
    const el = document.createElement('span');
    el.className = 'cm-image-chip';
    el.title = this.src;
    const tag = el.appendChild(document.createElement('span'));
    tag.className = 'cm-image-tag';
    tag.textContent = 'IMG';
    el.appendChild(document.createTextNode(this.alt || this.src));
    return el;
  }
}

class CodeLabelWidget extends WidgetType {
  constructor(readonly lang: string) {
    super();
  }
  override eq(other: CodeLabelWidget) {
    return other.lang === this.lang;
  }
  toDOM() {
    const el = document.createElement('span');
    el.className = 'cm-code-lang';
    el.textContent = this.lang;
    return el;
  }
}

// ── Builder ──────────────────────────────────────────────────────────────────

class Builder {
  readonly out: Range<Decoration>[] = [];

  constructor(
    readonly state: EditorState,
    readonly focused: boolean,
  ) {}

  /** A range is "active" (shown raw) when a selection touches its lines. */
  active(from: number, to: number): boolean {
    if (!this.focused) return false;
    const doc = this.state.doc;
    const start = doc.lineAt(from).from;
    const end = doc.lineAt(to).to;
    return this.state.selection.ranges.some((r) => r.to >= start && r.from <= end);
  }

  touches(from: number, to: number): boolean {
    return this.focused && this.state.selection.ranges.some((r) => r.to >= from && r.from <= to);
  }

  hide(from: number, to: number) {
    if (from >= to) return;
    const doc = this.state.doc;
    if (doc.lineAt(from).number !== doc.lineAt(to).number) return;
    this.out.push(hiddenMark.range(from, to));
  }

  replace(from: number, to: number, widget: WidgetType) {
    if (from >= to) return;
    this.out.push(Decoration.replace({ widget }).range(from, to));
  }

  mark(from: number, to: number, cls: string) {
    if (from < to) this.out.push(mark(cls).range(from, to));
  }

  lines(from: number, to: number, cls: (first: boolean, last: boolean) => string) {
    const doc = this.state.doc;
    const first = doc.lineAt(from).number;
    const last = doc.lineAt(to).number;
    for (let n = first; n <= last; n++) {
      this.out.push(line(cls(n === first, n === last)).range(doc.line(n).from));
    }
  }

  charAt(pos: number) {
    return this.state.doc.sliceString(pos, pos + 1);
  }

  text(node: { from: number; to: number }) {
    return this.state.doc.sliceString(node.from, node.to);
  }
}

const HEADING = /^(ATX|Setext)Heading([1-6])$/;
const LINKISH = new Set(['Link', 'Image', 'Autolink']);

function visit(b: Builder, ref: SyntaxNodeRef): boolean | void {
  const { name, from, to } = ref;
  const heading = HEADING.exec(name);
  if (heading) {
    const level = heading[2];
    if (heading[1] === 'ATX') {
      b.lines(from, from, () => `cm-h cm-h${level}`);
    } else {
      b.lines(from, to, (first, last) =>
        last && !first ? 'cm-setext-underline' : `cm-h cm-h${level}`,
      );
    }
    return;
  }

  switch (name) {
    case 'HeaderMark': {
      if (b.active(from, to)) return;
      const parent = ref.node.parent;
      if (!parent || parent.name.startsWith('Setext')) return b.hide(from, to);
      if (from === parent.from) return b.hide(from, b.charAt(to) === ' ' ? to + 1 : to);
      return b.hide(b.charAt(from - 1) === ' ' ? from - 1 : from, to);
    }

    case 'EmphasisMark':
    case 'StrikethroughMark':
      if (!b.active(from, to)) b.hide(from, to);
      return;

    case 'InlineCode':
      b.mark(from, to, 'cm-inline-code');
      return;

    case 'CodeMark':
      if (ref.node.parent?.name === 'InlineCode' && !b.active(from, to)) b.hide(from, to);
      return;

    case 'Escape':
      if (!b.active(from, to)) b.hide(from, from + 1);
      return;

    case 'Link': {
      const node = ref.node;
      const url = node.getChild('URL');
      const href = url ? b.text(url) : null;
      b.out.push(
        Decoration.mark({
          class: 'cm-link',
          attributes: href ? { 'data-href': href, title: `${href}\nCtrl+clic para abrir` } : {},
        }).range(from, to),
      );
      if (!b.active(from, to)) {
        const marks = node.getChildren('LinkMark');
        const open = marks[0];
        const close = marks[1];
        if (open) b.hide(open.from, open.to);
        if (close) b.hide(close.from, to);
      }
      return;
    }

    case 'Autolink': {
      const url = ref.node.getChild('URL');
      if (url) b.out.push(Decoration.mark({ class: 'cm-link', attributes: { 'data-href': b.text(url) } }).range(url.from, url.to));
      if (!b.active(from, to)) {
        for (const m of ref.node.getChildren('LinkMark')) b.hide(m.from, m.to);
      }
      return false;
    }

    case 'URL':
      if (!LINKISH.has(ref.node.parent?.name ?? '')) {
        b.out.push(Decoration.mark({ class: 'cm-link', attributes: { 'data-href': b.text(ref) } }).range(from, to));
      }
      return;

    case 'Image': {
      if (b.active(from, to)) return;
      const node = ref.node;
      const marks = node.getChildren('LinkMark');
      const alt = marks.length >= 2 ? b.state.doc.sliceString(marks[0]!.to, marks[1]!.from) : '';
      const url = node.getChild('URL');
      const doc = b.state.doc;
      if (doc.lineAt(from).number === doc.lineAt(to).number) {
        b.replace(from, to, new ImageWidget(alt, url ? b.text(url) : ''));
      }
      return false;
    }

    case 'Blockquote':
      b.lines(from, to, () => 'cm-quote');
      return;

    case 'QuoteMark':
      if (!b.active(from, to)) b.hide(from, b.charAt(to) === ' ' ? to + 1 : to);
      return;

    case 'ListMark': {
      const item = ref.node.parent;
      const list = item?.parent?.name;
      // Leading spaces of nested items are nearly invisible in a
      // proportional font; give them a real indent.
      const lineStart = b.state.doc.lineAt(from).from;
      if (from > lineStart && /^\s+$/.test(b.state.doc.sliceString(lineStart, from))) {
        b.mark(lineStart, from, 'cm-list-indent');
      }
      if (list === 'BulletList') {
        if (item?.getChild('Task')) {
          if (!b.active(from, to)) b.hide(from, b.charAt(to) === ' ' ? to + 1 : to);
        } else if (!b.active(from, to)) {
          b.replace(from, to, new BulletWidget());
        }
      } else {
        b.mark(from, to, 'cm-list-number');
      }
      return;
    }

    case 'TaskMarker': {
      const checked = /[xX]/.test(b.charAt(from + 1));
      const lineInfo = b.state.doc.lineAt(from);
      b.lines(from, from, () => 'cm-task-line');
      if (!b.touches(from, to)) b.replace(from, to, new CheckboxWidget(checked, from));
      if (checked) b.mark(Math.min(to + 1, lineInfo.to), lineInfo.to, 'cm-task-done');
      return;
    }

    case 'HorizontalRule':
      b.lines(from, from, () => 'cm-hr-line');
      if (!b.active(from, to)) b.replace(from, to, new RuleWidget());
      return;

    case 'FencedCode':
      fencedCode(b, ref.node);
      return false;

    case 'CodeBlock':
      b.lines(from, to, () => 'cm-code cm-code-indented');
      return false;

    case 'Table':
      b.lines(from, to, (first) => (first ? 'cm-table cm-table-head' : 'cm-table'));
      return;

    case 'TableDelimiter':
      b.mark(from, to, 'cm-table-delim');
      return;

    case 'Frontmatter':
      b.lines(from, to, (first, last) => `cm-frontmatter${first ? ' cm-frontmatter-first' : ''}${last ? ' cm-frontmatter-last' : ''}`);
      return false;

    case 'HTMLBlock':
    case 'CommentBlock':
      b.lines(from, to, () => 'cm-html-block');
      return false;

    case 'HTMLTag':
    case 'Comment':
      b.mark(from, to, 'cm-html');
      return;
  }
}

function fencedCode(b: Builder, node: SyntaxNode) {
  const doc = b.state.doc;
  const { from, to } = node;
  b.lines(from, to, (first, last) => `cm-code${first ? ' cm-code-first' : ''}${last ? ' cm-code-last' : ''}`);
  if (b.active(from, to)) return;

  const marks = node.getChildren('CodeMark');
  const open = marks[0];
  if (!open) return;
  const firstLine = doc.lineAt(open.from);
  const info = node.getChild('CodeInfo');
  b.replace(open.from, firstLine.to, new CodeLabelWidget(info ? b.text(info).toLowerCase() : ''));

  const close = marks.length > 1 ? marks[marks.length - 1] : undefined;
  if (close && doc.lineAt(close.from).number !== firstLine.number) b.hide(close.from, close.to);
}

function build(view: EditorView): DecorationSet {
  const b = new Builder(view.state, view.hasFocus);
  const tree = syntaxTree(view.state);
  for (const { from, to } of view.visibleRanges) {
    tree.iterate({ from, to, enter: (ref) => visit(b, ref) });
  }
  return Decoration.set(b.out, true);
}

// ── Extension ────────────────────────────────────────────────────────────────

export function livePreview(options: LivePreviewOptions): Extension {
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      constructor(view: EditorView) {
        this.decorations = build(view);
      }
      update(u: ViewUpdate) {
        if (
          u.docChanged ||
          u.viewportChanged ||
          u.selectionSet ||
          u.focusChanged ||
          syntaxTree(u.startState) !== syntaxTree(u.state)
        ) {
          this.decorations = build(u.view);
        }
      }
    },
    { decorations: (v) => v.decorations },
  );

  const events = EditorView.domEventHandlers({
    mousedown(event, view) {
      const target = event.target as HTMLElement;
      const box = target.closest<HTMLElement>('.cm-task-box');
      if (box?.dataset.pos) {
        const pos = Number(box.dataset.pos);
        const checked = box.classList.contains('is-checked');
        view.dispatch({
          changes: { from: pos + 1, to: pos + 2, insert: checked ? ' ' : 'x' },
          userEvent: 'input.toggle',
        });
        event.preventDefault();
        return true;
      }
      const link = target.closest<HTMLElement>('[data-href]');
      if (link?.dataset.href && (event.ctrlKey || event.metaKey)) {
        options.onOpenLink(link.dataset.href);
        event.preventDefault();
        return true;
      }
      return false;
    },
  });

  return [plugin, events, EditorView.editorAttributes.of({ class: 'cm-live' })];
}
