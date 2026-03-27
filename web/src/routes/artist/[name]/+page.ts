import type { PageLoad } from './$types';

export const load: PageLoad = ({ params }) => {
  return {
    name: decodeURIComponent(params.name)
  };
};