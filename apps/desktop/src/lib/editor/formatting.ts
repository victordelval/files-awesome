import { EditorSelection, type StateCommand } from '@codemirror/state';

/** Wrap / unwrap each selection with `marker` (e.g. `**` for bold). */
export function toggleWrap(marker: string): StateCommand {
  const m = marker.length;
  return ({ state, dispatch }) => {
    const tr = state.changeByRange((range) => {
      const { from, to } = range;
      const before = state.sliceDoc(from - m, from);
      const after = state.sliceDoc(to, to + m);
      if (before === marker && after === marker) {
        return {
          changes: [
            { from: from - m, to: from },
            { from: to, to: to + m },
          ],
          range: EditorSelection.range(from - m, to - m),
        };
      }
      const selected = state.sliceDoc(from, to);
      if (selected.length >= 2 * m && selected.startsWith(marker) && selected.endsWith(marker)) {
        return {
          changes: [
            { from, to: from + m },
            { from: to - m, to },
          ],
          range: EditorSelection.range(from, to - 2 * m),
        };
      }
      return {
        changes: [
          { from, insert: marker },
          { from: to, insert: marker },
        ],
        range: EditorSelection.range(from + m, to + m),
      };
    });
    dispatch(state.update(tr, { scrollIntoView: true, userEvent: 'input.format' }));
    return true;
  };
}

const HEADING_PREFIX = /^(\s{0,3})(#{1,6})(\s+|$)/;

/** Set the heading level of every selected line (same level again → paragraph). */
export function setHeading(level: number): StateCommand {
  return ({ state, dispatch }) => {
    const changes = [];
    const seen = new Set<number>();
    for (const range of state.selection.ranges) {
      for (let pos = range.from; pos <= range.to; ) {
        const line = state.doc.lineAt(pos);
        pos = line.to + 1;
        if (seen.has(line.number)) continue;
        seen.add(line.number);
        const match = HEADING_PREFIX.exec(line.text);
        const current = match ? match[2]!.length : 0;
        const prefixEnd = match ? line.from + match[0].length : line.from;
        const target = current === level ? 0 : level;
        changes.push({
          from: line.from,
          to: prefixEnd,
          insert: target ? '#'.repeat(target) + ' ' : '',
        });
      }
    }
    dispatch(state.update({ changes, scrollIntoView: true, userEvent: 'input.format' }));
    return true;
  };
}

/** `[selection](|)` — caret lands where the URL goes. */
export const insertLink: StateCommand = ({ state, dispatch }) => {
  const tr = state.changeByRange((range) => {
    const text = state.sliceDoc(range.from, range.to);
    const insert = `[${text}]()`;
    const caret = text ? range.from + insert.length - 1 : range.from + 1;
    return {
      changes: { from: range.from, to: range.to, insert },
      range: EditorSelection.cursor(caret),
    };
  });
  dispatch(state.update(tr, { scrollIntoView: true, userEvent: 'input.format' }));
  return true;
};
