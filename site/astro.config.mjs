import { defineConfig } from 'astro/config';

const REPO = 'https://github.com/Niiihuel/limn';

// The guides are written to be read on GitHub: `layout.md#gaps` there is
// `/docs/layout/#gaps` here, and a path into the repository is a link to it.
function guideLinks() {
  const visit = (node) => {
    if (node.type === 'link' && !/^(https?:|#|\/|mailto:)/.test(node.url)) {
      const guide = node.url.match(/^(components\/)?([\w-]+)\.md(#.*)?$/);
      if (guide?.[1]) node.url = `/components/${guide[2]}/` + (guide[3] ?? '');
      else if (guide) node.url = (guide[2] === 'README' ? '/docs/' : `/docs/${guide[2]}/`) + (guide[3] ?? '');
      else node.url = `${REPO}/blob/main/${node.url.replace(/^(\.\.\/)+/, '')}`;
    }
    node.children?.forEach(visit);
  };
  return visit;
}

export default defineConfig({
  markdown: {
    remarkPlugins: [guideLinks],
    shikiConfig: { themes: { light: 'github-light', dark: 'github-dark-default' } },
  },
});
