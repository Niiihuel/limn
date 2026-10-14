export const REPO = 'https://github.com/Niiihuel/limn';

/** Guides in reading order; the slug is the file name in docs/. */
export const GUIDES = [
  { group: 'Getting started', items: [['getting-started', 'Installation'], ['concepts', 'Concepts']] },
  {
    group: 'Design system',
    items: [
      ['theming', 'Colors and themes'],
      ['tokens', 'Design tokens'],
      ['layout', 'Layout'],
      ['animation', 'Animation'],
    ],
  },
  {
    group: 'Guides',
    items: [
      ['components', 'Component API overview'],
      ['accessibility', 'Accessibility'],
      ['hot-reload', 'Hot reload'],
      ['tours', 'Tours'],
      ['liquid-glass', 'Liquid glass'],
    ],
  },
  {
    group: 'Contributing',
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
export const FOUNDATIONS = [
  { id: 'colors', title: 'Colors and themes', guide: 'theming', page: 'scheme', description: 'Theme, semantic color roles, and palettes for each surface.' },
  { id: 'tokens', title: 'Design tokens', guide: 'tokens', page: 'tokens', description: 'The radius scale and animation durations shared by the library.' },
] as const;

const patternIds = new Set(['editor', 'picker']);
const foundationIds = new Set<string>(FOUNDATIONS.map(({ page }) => page));

/** The gallery stays unchanged; the website separates foundations and compositions. */
export const CATALOG = pages;
export const COMPONENTS = pages.filter(({ id }) => !foundationIds.has(id) && !patternIds.has(id));
export const PATTERNS = pages.filter(({ id }) => patternIds.has(id));

export const guideHref = (slug: string) => {
  const foundation = FOUNDATIONS.find(({ guide }) => guide === slug);
  return foundation ? `/design-system/${foundation.id}/` : `/docs/${slug}/`;
};

/** The anchor of a section's first example on its component's page. */
export const exampleId = (title: string) => `example-${title.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')}`;

export const groups = <T extends { group: string }>(items: readonly T[]) =>
  [...new Set(items.map((item) => item.group))].map((group) => ({
    group,
    items: items.filter((item) => item.group === group),
  }));

export const DOC_NAV = [
  ...GUIDES.filter(({ group }) => group !== 'Contributing').map(({ group, items }) => ({
    group,
    items: [
      ...(group === 'Getting started' ? [{ href: '/docs/', label: 'Overview' }] : []),
      ...(group === 'Design system' ? [{ href: '/design-system/', label: 'Foundations' }] : []),
      ...items.map(([slug, label]) => ({ href: guideHref(slug), label })),
    ],
  })),
  { group: 'Components', items: [{ href: '/components/', label: 'All components' }] },
  ...groups(COMPONENTS).map(({ group, items }) => ({
    group: group === 'System' ? 'Primitives' : group,
    items: items.map(({ id, title, sections, new: fresh }) => ({
      href: `/components/${id}/`,
      label: title,
      // Marked in the menu while the release it came in is recent.
      fresh,
      // A component shown in several sections has a branch for each.
      children: sections.length > 1 ? sections.filter(({ title }) => title).map(({ title }) => ({ href: `/components/${id}/#${exampleId(title)}`, label: title })) : [],
    })),
  })),
  { group: 'Patterns', items: [
    { href: '/patterns/', label: 'All patterns' },
    ...PATTERNS.map(({ id, title }) => ({ href: `/components/${id}/`, label: title })),
  ] },
  ...GUIDES.filter(({ group }) => group === 'Contributing').map(({ group, items }) => ({
    group,
    items: items.map(([slug, label]) => ({ href: guideHref(slug), label })),
  })),
];
