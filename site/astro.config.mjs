import { defineConfig } from 'astro/config';

const REPO = 'https://github.com/Niiihuel/limn';

// The guides are written to be read on GitHub: `layout.md#gaps` there is
// `/docs/layout/#gaps` here, and a path into the repository is a link to it.
function guideLinks() {
  const page = (dir, name, hash = '') => (name === 'README' ? '/docs/' : `/${dir}/${name}/`) + hash;
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

export default defineConfig({
  markdown: {
    remarkPlugins: [guideLinks],
    shikiConfig: { themes: { light: 'github-light', dark: 'github-dark-default' } },
  },
});
