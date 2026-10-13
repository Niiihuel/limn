import { DOC_NAV } from '../data/site';

/** Everything the site's search can open: each page of the menu, and each section of a component. */
export const GET = () =>
  new Response(
    JSON.stringify(
      DOC_NAV.flatMap(({ group, items }) =>
        items.flatMap((item) => [
          { href: item.href, label: item.label, group },
          ...('children' in item ? item.children : []).map((child) => ({ href: child.href, label: child.label, group: item.label })),
        ]),
      ),
    ),
    { headers: { 'content-type': 'application/json' } },
  );
