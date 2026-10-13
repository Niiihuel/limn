import { defineConfig } from 'astro/config';

const REPO = 'https://github.com/Niiihuel/limn';

// The guides are written to be read on GitHub: `layout.md#gaps` there is
// `/docs/layout/#gaps` here, and a path into the repository is a link to it.
function guideLinks() {
  const page = (dir, name, hash = '') => {
    if (name === 'tokens') return `/design-system/tokens/${hash}`;
    if ((dir === 'docs' && name === 'theming') || (dir === 'components' && name === 'scheme')) {
      return `/design-system/colors/${hash}`;
    }
    return (name === 'README' ? '/docs/' : `/${dir}/${name}/`) + hash;
  };
  return (tree, file) => {
    // A component's guide links to its neighbours by name and to the general guides one level up.
    const inComponents = /[\\/]docs[\\/]components[\\/]/.test(file.path ?? '');
    const visit = (node) => {
      if (node.type === 'link' && !/^(https?:|#|\/|mailto:)/.test(node.url)) {
        const guide = node.url.match(/^(\.\.\/|components\/)?([\w-]+)\.md(#.*)?$/);
        if (!guide) node.url = `${REPO}/blob/main/${node.url.replace(/^(\.\.\/)+/, '')}`;
        else if (guide[1] === 'components/' || (inComponents && !guide[1])) node.url = page('components', guide[2], guide[3]);
        else node.url = page('docs', guide[2], guide[3]);
      }
      node.children?.forEach(visit);
    };
    visit(tree);
  };
}

// In a table of keys, each key is a keycap: `Left, Right` and `Ctrl+Enter`.
function keycaps() {
  const text = (node) => node.value ?? node.children?.map(text).join('') ?? '';
  const escape = (value) => value.replace(/&/g, '&amp;').replace(/</g, '&lt;');
  const visit = (node) => {
    if (node.type === 'table' && /^keys?$/i.test(text(node.children[0]?.children[0] ?? {}).trim())) {
      for (const row of node.children.slice(1)) {
        const cell = row.children[0];
        const keys = text(cell).split(/(\s*,\s*|\s+or\s+|\s*\/\s*|\+)/);
        // A cell that describes keys in words, not names them, stays prose.
        if (keys.some((part, index) => index % 2 === 0 && !/^[A-Z←→↑↓][\w ]{0,11}$/.test(part.trim()))) continue;
        const html = keys.map((part, index) => (index % 2 ? escape(part) : part.trim() && `<kbd>${escape(part.trim())}</kbd>`)).join('');
        cell.children = [{ type: 'html', value: html }];
      }
    }
    node.children?.forEach(visit);
  };
  return visit;
}

// A `mermaid` block is drawn by limn itself, in the gallery's frame, as the
// library draws one inside its own Markdown.
function diagrams() {
  const attribute = (value) => value.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');
  const visit = (node) => {
    node.children?.forEach((child, index) => {
      if (child.type === 'code' && child.lang === 'mermaid') {
        node.children[index] = { type: 'html', value: `<iframe class="live" title="Diagram" data-diagram="${attribute(child.value)}" style="height:140px"></iframe>` };
      } else visit(child);
    });
  };
  return visit;
}

export default defineConfig({
  markdown: {
    remarkPlugins: [guideLinks, keycaps, diagrams],
    shikiConfig: { themes: { light: 'github-light', dark: 'github-dark-default' } },
  },
});
