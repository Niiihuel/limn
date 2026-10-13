/**
 * A guide's first paragraph, which says what the component is for, as plain
 * text: the page prints it as its lead and as its description, not as Markdown.
 */
export const lead = (markdown?: string) =>
  markdown
    ?.split(/\n\s*\n/)
    .find((block) => !block.startsWith('#'))
    ?.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[`*]/g, '')
    .replace(/\s+/g, ' ')
    .trim() ?? '';
