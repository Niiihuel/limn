/** A guide's first paragraph, which says what the component is for. */
export const lead = (markdown?: string) =>
  markdown
    ?.split(/\n\s*\n/)
    .find((block) => !block.startsWith('#'))
    ?.replace(/\s+/g, ' ')
    .trim() ?? '';
