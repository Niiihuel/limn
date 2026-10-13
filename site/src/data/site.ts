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

/**
 * One entry per page of the gallery (crates/limn-gallery/src/pages): `id` is
 * the page's ID there, which is also the fragment the embedded gallery opens.
 */
export const COMPONENTS = [
  { group: 'System', id: 'tokens', title: 'Shape and motion', about: 'Corner radii and pace for the whole application, from one set of tokens.', guide: '/docs/tokens/' },
  { group: 'System', id: 'scheme', title: 'Color scheme', about: 'A palette derived from three colors, light and dark.', guide: '/docs/theming/' },
  { group: 'System', id: 'typography', title: 'Text and separators', about: 'Text styles, weights, links and dividers.', guide: '/docs/components/#text' },
  { group: 'Controls', id: 'buttons', title: 'Buttons', about: 'Eleven variants, sizes down to 22 px, icons, loading and disabled states.', guide: '/docs/components/#actions' },
  { group: 'Controls', id: 'inputs', title: 'Inputs', about: 'Text fields, toggles, sliders, pickers and the controls of a form.', guide: '/docs/components/#inputs' },
  { group: 'Controls', id: 'navigation', title: 'Navigation', about: 'Tabs, breadcrumbs, steppers and rows that take you somewhere.', guide: '/docs/components/#navigation' },
  { group: 'Controls', id: 'lists', title: 'Lists', about: 'Uniform and virtual lists, trees and rows that open in place.', guide: '/docs/components/#data' },
  { group: 'Controls', id: 'table', title: 'Table', about: 'Rows of cells under a header that sorts, resizes and hides columns.', guide: '/docs/components/#table' },
  { group: 'Controls', id: 'preferences', title: 'Preferences', about: 'Grouped settings rows with their controls.', guide: '/docs/components/#layout-and-grouping' },
  { group: 'Controls', id: 'page', title: 'Page and sections', about: 'The frame of a page: headings, sections and their spacing.', guide: '/docs/components/#layout-and-grouping' },
  { group: 'Controls', id: 'panels', title: 'Panels and pager', about: 'Resizable panels, collapsibles and a pager.', guide: '/docs/components/#layout-and-grouping' },
  { group: 'Controls', id: 'blocks', title: 'Cards and blocks', about: 'Cards, framed cards, media tiles and the blocks built on them.', guide: '/docs/components/#layout-and-grouping' },
  { group: 'Controls', id: 'markdown', title: 'Markdown', about: 'Markdown rendered natively: tables, code, links, images and diagrams.', guide: '/docs/components/#markdown' },
  { group: 'Controls', id: 'diagrams', title: 'Diagrams', about: 'Mermaid flowcharts and sequence diagrams drawn without a browser.', guide: '/docs/components/#markdown' },
  { group: 'Controls', id: 'messages', title: 'Message actions', about: 'The actions under a message in a conversation.', guide: '/docs/components/#actions' },
  { group: 'Controls', id: 'chrome', title: 'Menu and status bars', about: 'A window menu bar and a status bar with divided cells.', guide: '/docs/components/#navigation' },
  { group: 'Controls', id: 'glass', title: 'Liquid glass', about: 'A material that blurs, tints and refracts the scene behind a surface.', guide: '/docs/liquid-glass/' },
  { group: 'Controls', id: 'editor', title: 'Editor and composer', about: 'A code editor on a rope, with folding, a minimap, search and Vim keys.', guide: '/docs/components/#code' },
  { group: 'States', id: 'empty', title: 'Empty state', about: 'What a view shows when it has nothing to show.', guide: '/docs/components/#feedback-and-status' },
  { group: 'States', id: 'feedback', title: 'Notices and loading', about: 'Alerts, toasts, spinners, skeletons and shimmer.', guide: '/docs/components/#feedback-and-status' },
  { group: 'States', id: 'context', title: 'Context window', about: 'A card whose figures count and whose cells fill and empty.', guide: '/docs/components/#feedback-and-status' },
  { group: 'Overlays', id: 'overlays', title: 'Menus and dialogs', about: 'Dropdown and context menus, modals and alert dialogs.', guide: '/docs/components/#overlays' },
  { group: 'Overlays', id: 'menus', title: 'Pickers and suggestions', about: 'Selects, comboboxes, date pickers and suggestion lists.', guide: '/docs/components/#overlays' },
  { group: 'Overlays', id: 'tooltip', title: 'Tooltip', about: 'Hints with a title, a description and a shortcut.', guide: '/docs/components/#overlays' },
  { group: 'Overlays', id: 'tour', title: 'Guided tour', about: 'Coach marks that point at controls and explain them one at a time.', guide: '/docs/tours/' },
] as const;

export const groups = <T extends { group: string }>(items: readonly T[]) =>
  [...new Set(items.map((item) => item.group))].map((group) => ({
    group,
    items: items.filter((item) => item.group === group),
  }));
