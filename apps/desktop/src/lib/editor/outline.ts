import { ensureSyntaxTree, syntaxTree } from '@codemirror/language';
import type { EditorState } from '@codemirror/state';

export interface Heading {
  level: number;
  text: string;
  from: number;
  /** "01", "02"… for top-level sections, like a drawing index. */
  num: string | null;
}

const HEADING = /^(ATX|Setext)Heading([1-6])$/;

function clean(raw: string, atx: boolean): string {
  let text = atx
    ? raw.replace(/^\s{0,3}#{1,6}\s*/, '').replace(/\s+#+\s*$/, '')
    : (raw.split('\n')[0] ?? '');
  text = text
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[*_`~]/g, '')
    .trim();
  return text || '—';
}

export function extractOutline(state: EditorState): Heading[] {
  const tree = ensureSyntaxTree(state, state.doc.length, 100) ?? syntaxTree(state);
  const found: Omit<Heading, 'num'>[] = [];
  tree.iterate({
    enter(node) {
      const m = HEADING.exec(node.name);
      if (m) {
        found.push({
          level: Number(m[2]),
          text: clean(state.doc.sliceString(node.from, node.to), m[1] === 'ATX'),
          from: node.from,
        });
        return false;
      }
      if (node.name === 'FencedCode' || node.name === 'Frontmatter') return false;
      return undefined;
    },
  });
  const top = Math.min(...found.map((h) => h.level));
  let n = 0;
  return found.map((h) => ({
    ...h,
    num: h.level === top ? String(++n).padStart(2, '0') : null,
  }));
}

/** Index of the heading whose section contains `pos`, or -1. */
export function headingAt(outline: Heading[], pos: number): number {
  let lo = 0;
  let hi = outline.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (outline[mid]!.from <= pos) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return found;
}

const WORD = /[\p{L}\p{N}]+(?:['’\-][\p{L}\p{N}]+)*/gu;

export function countWords(text: string): number {
  let n = 0;
  for (const _ of text.matchAll(WORD)) n++;
  return n;
}
