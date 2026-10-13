import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

/** Where a call's parentheses close, skipping strings, characters and comments. */
const closing = (source: string, open: number) => {
  let depth = 0;
  for (let at = open; at < source.length; at++) {
    const char = source[at];
    if (char === '"') {
      for (at++; at < source.length && source[at] !== '"'; at++) if (source[at] === '\\') at++;
    } else if (char === "'" && (source[at + 1] === '\\' || source[at + 2] === "'")) {
      at = source.indexOf("'", at + 2);
    } else if (char === '/' && source[at + 1] === '/') {
      at = source.indexOf('\n', at);
    } else if ('([{'.includes(char)) {
      depth++;
    } else if (')]}'.includes(char) && --depth === 0) {
      return at;
    }
  }
  return -1;
};

const dedent = (code: string) => {
  const lines = code.replace(/^\n+|\s+$/g, '').split('\n');
  const margin = Math.min(...lines.filter((line) => line.trim()).map((line) => line.match(/^ */)![0].length));
  return lines.map((line) => line.slice(margin)).join('\n');
};

/**
 * What the gallery draws under a section's title: the demos of
 * `section(cx, "Title", vec![…])` in the page's module, as they are written
 * there. Empty when the module does not say the title in so many words.
 */
export const sectionSource = (module: string, title: string) => {
  if (!title) return '';
  let source: string;
  try {
    // The site is built from its own directory, beside `crates`.
    source = readFileSync(resolve(process.cwd(), '../crates/limn-gallery/src/pages', `${module}.rs`), 'utf8');
  } catch {
    return '';
  }
  const head = `section(cx, ${JSON.stringify(title)},`;
  const call = source.indexOf(head);
  if (call < 0) return '';
  const close = closing(source, source.indexOf('(', call));
  if (close < 0) return '';
  const demos = source.slice(call + head.length, close).trim().replace(/,$/, '');
  const list = demos.match(/^vec!\[([\s\S]*)\]$/);
  return dedent(list ? list[1] : demos);
};

/** The items of a list of demos, cut at the commas that are not inside anything. */
const items = (code: string) => {
  const found: string[] = [];
  let start = 0;
  for (let at = 0; at < code.length; at++) {
    const char = code[at];
    if ('([{'.includes(char)) {
      at = closing(code, at);
      if (at < 0) return [];
    } else if (char === '"') {
      for (at++; at < code.length && code[at] !== '"'; at++) if (code[at] === '\\') at++;
    } else if (char === '/' && code[at + 1] === '/') {
      at = code.indexOf('\n', at);
      if (at < 0) break;
    } else if (char === ',') {
      found.push(code.slice(start, at));
      start = at + 1;
    }
  }
  if (code.slice(start).trim()) found.push(code.slice(start));
  return found;
};

/**
 * A section's source, one piece for each of its variants: the gallery groups
 * a demo with its captions, before it or after it, and so does this. Empty
 * when the pieces do not come to `count`, as when a demo is built elsewhere.
 */
export const variantSources = (module: string, title: string, count: number) => {
  const all = items(sectionSource(module, title));
  const caption = (item: string) => /^\s*(\/\/[^\n]*\n\s*)*caption\(/.test(item);
  const leading = all.length > 0 && caption(all[0]);
  const groups: string[][] = [];
  for (const item of all) {
    const last = groups.at(-1);
    if (last && !leading && caption(item)) last.push(item);
    else if (last && leading && (!caption(item) || last.every(caption))) last.push(item);
    else groups.push([item]);
  }
  if (groups.length !== count) return [];
  return groups.map((group) => dedent(group.filter((item) => !caption(item)).join(',')));
};
