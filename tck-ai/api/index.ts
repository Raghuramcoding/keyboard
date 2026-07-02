/**
 * Vercel serverless function — wraps the Hono app.
 *
 * This file is the entry point for Vercel deployments. Each invocation
 * creates a fresh Hono instance to handle the request.
 */

import app from "../src/server.js";

export default app;
