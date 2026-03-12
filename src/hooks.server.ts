import { execSync } from 'child_process';
import type { Handle } from '@sveltejs/kit';

// In production, apply the DB schema on server startup so it's always up to date.
// In dev, this is handled by the `prisma db push` step in the npm dev script.
if (process.env.NODE_ENV === 'production') {
	try {
		execSync('node node_modules/prisma/build/index.js db push', {
			cwd: process.cwd(),
			env: process.env,
			stdio: 'pipe'
		});
	} catch (err) {
		console.error('[hooks.server] prisma db push failed:', err);
	}
}

export const handle: Handle = async ({ event, resolve }) => {
	return resolve(event);
};
