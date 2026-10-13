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

/** The gallery's groups, which are in Spanish like the rest of its text. */
const GROUPS: Record<string, string> = {
  Sistema: 'System',
  Acciones: 'Actions',
  Entradas: 'Inputs',
  Navegación: 'Navigation',
  Datos: 'Data',
  Estructura: 'Layout',
  Estados: 'States',
  Superposiciones: 'Overlays',
};

/**
 * One entry per gallery page: the gallery's own list, printed by
 * `limn-gallery --pages` into pages.json. `id` is the page there, the address
 * here and the name of the guide in docs/components.
 */
export const COMPONENTS = (pages as { id: string; group: string; title: string; source: string }[]).map((page) => ({
  ...page,
  group: GROUPS[page.group] ?? page.group,
}));

export const groups = <T extends { group: string }>(items: readonly T[]) =>
  [...new Set(items.map((item) => item.group))].map((group) => ({
    group,
    items: items.filter((item) => item.group === group),
  }));
