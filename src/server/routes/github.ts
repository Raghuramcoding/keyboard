import { Hono } from 'hono';
import { Octokit } from '@octokit/rest';

export const githubRoutes = new Hono();

githubRoutes.get('/auth', (c) => {
  const clientId = process.env.TCK_GITHUB_CLIENT_ID;
  const redirectUrl = encodeURIComponent(`http://localhost:3000/api/github/callback`);
  return c.redirect(`https://github.com/login/oauth/authorize?client_id=${clientId}&redirect_uri=${redirectUrl}&scope=repo,user`);
});

githubRoutes.get('/callback', async (c) => {
  const code = c.req.query('code');
  if (!code) return c.json({ error: "No code provided" }, 400);

  const clientId = process.env.TCK_GITHUB_CLIENT_ID;
  const clientSecret = process.env.TCK_GITHUB_CLIENT_SECRET;

  const resp = await fetch('https://github.com/login/oauth/access_token', {
    method: 'POST',
    headers: { 'Accept': 'application/json' },
    body: new URLSearchParams({
      client_id: clientId!,
      client_secret: clientSecret!,
      code: code,
    }),
  });

  const data = await resp.json();
  return c.json(data);
});

githubRoutes.get('/repos', async (c) => {
  const token = c.req.header('Authorization')?.replace('Bearer ', '');
  if (!token) return c.json({ error: "Unauthorized" }, 401);

  const octokit = new Octokit({ auth: token });
  const { data } = await octokit.repos.listForAuthenticatedUser();
  return c.json(data);
});
