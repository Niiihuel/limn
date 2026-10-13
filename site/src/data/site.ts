export const REPO = 'https://github.com/Niiihuel/limn';

/** Guides in reading order; the slug is the file name in docs/. */
export const GUIDES = [
  { group: 'Learn', items: [['getting-started', 'Getting started'], ['concepts', 'Concepts']] },
  {
    group: 'Build',
    items: [
      ['layout', 'Layout'],
      ['components', 'Components'],
      ['theming', 'Theming'],
      ['tokens', 'Tokens'],
      ['animation', 'Animation'],
      ['tours', 'Tours'],
      ['liquid-glass', 'Liquid glass'],
    ],
  },
  {
    group: 'Work on limn',
    items: [
      ['testing', 'Testing'],
      ['gallery', 'Gallery and hot reload'],
      ['contributing', 'Contributing a component'],
      ['backend', 'Backend contract'],
    ],
  },
] as const;

import pages from './pages.json';

/**
 * One entry per gallery page: the gallery's own list, printed by
 * `limn-gallery --pages` into pages.json. `id` is the page there, the address
 * here and the name of the guide in docs/components.
 */
export const COMPONENTS = pages;

export const groups = <T extends { group: string }>(items: readonly T[]) =>
  [...new Set(items.map((item) => item.group))].map((group) => ({
    group,
    items: items.filter((item) => item.group === group),
  }));
