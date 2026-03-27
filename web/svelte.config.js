import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      strict: false
    })
  }
};

// Add alias for TypeScript-only shared types
config.kit.alias = {
  '@player/shared': '../packages/shared/src/index.d.ts',
  '@player/shared/*': '../packages/shared/src/*'
};

export default config;
