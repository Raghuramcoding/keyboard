import { Hono } from 'hono';
import { ptyService } from '../services/pty.js';
import type { CreateInstanceRequest, UpdateInstanceRequest, InstanceInfo } from '../../core/types.js';

export const instanceRoutes = new Hono();
const instances: Map<string, InstanceInfo> = new Map();

instanceRoutes.post('/', async (c) => {
  const body = await c.req.json<CreateInstanceRequest>();
  const id = Math.random().toString(36).substring(2, 9);
  const info: InstanceInfo = {
    id,
    kind: body.kind,
    label: body.label || 'New Instance',
    command: body.command,
    cwd: body.cwd,
    status: 0, // Running
    created_at: Date.now(),
    agent_state: 0, // Idle
    last_output_at: Date.now(),
  };
  instances.set(id, info);
  return c.json(info);
});

instanceRoutes.get('/', (c) => {
  return c.json(Array.from(instances.values()));
});

instanceRoutes.patch(`/:id`, async (c) => {
  const id = c.req.param('id');
  const body = await c.req.json<UpdateInstanceRequest>();
  const inst = instances.get(id);
  if (!inst) return c.json({ error: "Not found" }, 404);
  if (body.label) inst.label = body.label;
  return c.json(inst);
});

instanceRoutes.delete(`/:id`, (c) => {
  const id = c.req.param('id');
  ptyService.removeInstance(id);
  instances.delete(id);
  return c.json({ success: true });
});
