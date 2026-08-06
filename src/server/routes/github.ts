import { Hono } from 'hono';
import { Octokit } from '@octokit/rest';

export const githubRoutes = new Hono();

function callbackUrl(requestUrl: string): string {
  if (process.env.TCK_GITHUB_CALLBACK) return process.env.TCK_GITHUB_CALLBACK;
  return new URL('/api/github/callback', requestUrl).toString();
}

githubRoutes.get('/auth', (c) => {
  const clientId = process.env.TCK_GITHUB_CLIENT_ID;
  if (!clientId) return c.json({ error: 'TCK_GITHUB_CLIENT_ID is not configured' }, 503);
  const redirectUrl = encodeURIComponent(callbackUrl(c.req.url));
  return c.redirect(`https://github.com/login/oauth/authorize?client_id=${clientId}&redirect_uri=${redirectUrl}&scope=repo,user`);
});

githubRoutes.get('/callback', async (c) => {
  const code = c.req.query('code');
  if (!code) return c.json({ error: "No code provided" }, 400);

  const clientId = process.env.TCK_GITHUB_CLIENT_ID;
  const clientSecret = process.env.TCK_GITHUB_CLIENT_SECRET;
  if (!clientId || !clientSecret) {
    return c.json({ error: 'GitHub OAuth is not configured' }, 503);
  }

  const resp = await fetch('https://github.com/login/oauth/access_token', {
    method: 'POST',
    headers: { 'Accept': 'application/json' },
    body: new URLSearchParams({
      client_id: clientId,
      client_secret: clientSecret,
      code,
      redirect_uri: callbackUrl(c.req.url),
    }),
  });

  const data = await resp.json() as { access_token?: string; error?: string };
  if (!data.access_token) {
    return c.json({ error: data.error ?? 'Failed to exchange OAuth code' }, 502);
  }
  return c.json({ access_token: data.access_token });
});

githubRoutes.get('/repos', async (c) => {
  const token = c.req.header('Authorization')?.replace('Bearer ', '');
  if (!token) return c.json({ error: "Unauthorized" }, 401);

  const octokit = new Octokit({ auth: token });
  const { data } = await octokit.repos.listForAuthenticatedUser();
  return c.json(data);
});
