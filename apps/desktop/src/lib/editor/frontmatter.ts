import type { MarkdownConfig } from '@lezer/markdown';

const FENCE = /^---\s*$/;
const CLOSE = /^(---|\.\.\.)\s*$/;
/** The line after the opening fence must look like YAML, otherwise a `---`
 *  at the top of a document is just a horizontal rule. */
const YAML_START = /^(#|[\w"'-][^:]*:(\s|$))/;

/**
 * Parses a YAML front-matter block (`---` … `---`) at the very start of a
 * document as a `Frontmatter` node instead of a setext heading / rule.
 */
export const Frontmatter: MarkdownConfig = {
  defineNodes: [{ name: 'Frontmatter', block: true }, { name: 'FrontmatterMark' }],
  parseBlock: [
    {
      name: 'Frontmatter',
      before: 'HorizontalRule',
      parse(cx, line) {
        if (cx.lineStart !== 0 || !FENCE.test(line.text) || !YAML_START.test(cx.peekLine())) {
          return false;
        }
        const start = cx.lineStart;
        const marks = [cx.elt('FrontmatterMark', start, start + 3)];
        let end = start + line.text.length;
        while (cx.nextLine()) {
          end = cx.lineStart + line.text.length;
          if (CLOSE.test(line.text)) {
            marks.push(cx.elt('FrontmatterMark', cx.lineStart, cx.lineStart + 3));
            cx.nextLine();
            break;
          }
        }
        cx.addElement(cx.elt('Frontmatter', start, end, marks));
        return true;
      },
    },
  ],
};
