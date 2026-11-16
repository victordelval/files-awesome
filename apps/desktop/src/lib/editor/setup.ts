import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { markdown, markdownKeymap, markdownLanguage } from '@codemirror/lang-markdown';
import { HighlightStyle, indentOnInput, syntaxHighlighting } from '@codemirror/language';
import { languages } from '@codemirror/language-data';
import { search, searchKeymap } from '@codemirror/search';
import { Compartment, EditorState, type Extension, Prec } from '@codemirror/state';
import {
  drawSelection,
  dropCursor,
  EditorView,
  keymap,
  placeholder,
  type ViewUpdate,
} from '@codemirror/view';
import { styleTags, tags as t } from '@lezer/highlight';
import { insertLink, setHeading, toggleWrap } from './formatting';
import { Frontmatter } from './frontmatter';
import { livePreview } from './livePreview';

export interface EditorHooks {
  onUpdate: (update: ViewUpdate) => void;
  onOpenLink: (href: string) => void;
}

/** Markdown + embedded-code highlighting, expressed as CSS classes so the
 *  palette lives entirely in `editor.css` (and follows light/dark tokens). */
const highlight = HighlightStyle.define([
  { tag: t.heading, class: 'tk-heading' },
  { tag: t.strong, class: 'tk-strong' },
  { tag: t.emphasis, class: 'tk-em' },
  { tag: t.strikethrough, class: 'tk-strike' },
  { tag: t.link, class: 'tk-link' },
  { tag: t.url, class: 'tk-url' },
  { tag: t.monospace, class: 'tk-mono' },
  { tag: t.quote, class: 'tk-quote' },
  { tag: [t.processingInstruction, t.contentSeparator], class: 'tk-mark' },
  { tag: t.labelName, class: 'tk-label' },
  { tag: [t.keyword, t.controlKeyword, t.moduleKeyword, t.operatorKeyword], class: 'tk-keyword' },
  { tag: [t.string, t.special(t.string), t.regexp], class: 'tk-string' },
  { tag: [t.number, t.bool, t.atom, t.null], class: 'tk-atom' },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], class: 'tk-comment' },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.definition(t.variableName)], class: 'tk-fn' },
  { tag: [t.typeName, t.className, t.namespace], class: 'tk-type' },
  { tag: [t.propertyName, t.attributeName], class: 'tk-prop' },
  { tag: [t.tagName, t.angleBracket], class: 'tk-tag' },
  { tag: [t.meta, t.documentMeta], class: 'tk-meta' },
  { tag: [t.punctuation, t.operator, t.separator, t.bracket], class: 'tk-punct' },
  { tag: t.invalid, class: 'tk-invalid' },
]);

const markdownSupport = markdown({
  base: markdownLanguage, // CommonMark + GFM (tables, tasks, strikethrough, autolinks)
  codeLanguages: languages,
  extensions: [
    Frontmatter,
    { props: [styleTags({ Frontmatter: t.meta, FrontmatterMark: t.processingInstruction })] },
  ],
});

const formattingKeymap = keymap.of([
  { key: 'Mod-b', run: toggleWrap('**') },
  { key: 'Mod-i', run: toggleWrap('*') },
  { key: 'Mod-e', run: toggleWrap('`') },
  { key: 'Mod-Shift-x', run: toggleWrap('~~') },
  { key: 'Mod-k', run: insertLink },
  ...[1, 2, 3, 4, 5, 6].map((n) => ({ key: `Mod-${n}`, run: setHeading(n) })),
]);

// Markdown has no line comments; free Mod-/ for the global "source" toggle.
const baseKeymap = defaultKeymap.filter((b) => b.key !== 'Mod-/' && b.key !== 'Alt-A');

export const modeCompartment = new Compartment();

export function modeExtension(source: boolean, hooks: EditorHooks): Extension {
  return source ? EditorView.editorAttributes.of({ class: 'cm-source' }) : livePreview(hooks);
}

export function createEditorState(doc: string, source: boolean, hooks: EditorHooks): EditorState {
  return EditorState.create({
    doc,
    extensions: [
      history(),
      drawSelection(),
      dropCursor(),
      indentOnInput(),
      EditorState.allowMultipleSelections.of(true),
      EditorView.lineWrapping,
      markdownSupport,
      syntaxHighlighting(highlight),
      search({ top: true }),
      placeholder('Empieza a escribir…'),
      EditorView.contentAttributes.of({ spellcheck: 'true', autocapitalize: 'off', 'aria-label': 'Documento' }),
      Prec.high(formattingKeymap),
      keymap.of([...markdownKeymap, ...baseKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
      modeCompartment.of(modeExtension(source, hooks)),
      EditorView.updateListener.of(hooks.onUpdate),
    ],
  });
}
