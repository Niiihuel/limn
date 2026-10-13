import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';

// The guides stay in the repository's docs/, where GitHub renders them too.
export const collections = {
  docs: defineCollection({ loader: glob({ pattern: '*.md', base: '../docs' }) }),
  // One guide per component, named after its page in the gallery.
  components: defineCollection({ loader: glob({ pattern: '*.md', base: '../docs/components' }) }),
};
