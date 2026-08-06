import { randomUUID } from 'node:crypto';
import { Hono } from 'hono';
import { ptyService } from '../services/pty.js';
import {
  AgentState,
  InstanceKind,
  InstanceStatus,
  type CreateInstanceRequest,
  type InstanceInfo,
  type UpdateInstanceRequest,
} from '../../core/types.js';

export const instanceRoutes = new Hono();
const instances: Map<string, InstanceInfo> = new Map();

instanceRoutes.post('/', async (c) => {
  const body = await c.req.json<CreateInstanceRequest>();
  const id = randomUUID();
  const info: InstanceInfo = {
    id,
    kind: body.kind ?? InstanceKind.Terminal,
    label: body.label || 'New Instance',
    command: body.command,
    cwd: body.cwd,
    status: InstanceStatus.Running,
    created_at: Date.now(),
    agent_state: AgentState.Idle,
    last_output_at: Date.now(),
  };

  if (info.kind === InstanceKind.Terminal) {
    try {
      ptyService.createInstance(id, info.command ?? '', info.cwd ?? '');
    } catch (err) {
      info.status = InstanceStatus.Error;
      info.agent_state = AgentState.Error;
      info.error = err instanceof Error ? err.message : 'Failed to start terminal';
    }
  }

  instances.set(id, info);
  return c.json(info, 201);
});

instanceRoutes.get('/', (c) => {
  return c.json(Array.from(instances.values()));
});

instanceRoutes.patch('/:id', async (c) => {
  const id = c.req.param('id');
  const inst = instances.get(id);
  if (!inst) return c.json({ error: 'Not found' }, 404);

  const body = await c.req.json<UpdateInstanceRequest>();
  if (body.label !== undefined) inst.label = body.label;
  if (body.status !== undefined) inst.status = body.status;
  if (body.agent_state !== undefined) inst.agent_state = body.agent_state;
  return c.json(inst);
});

instanceRoutes.delete('/:id', (c) => {
  const id = c.req.param('id');
  if (!instances.has(id)) return c.json({ error: 'Not found' }, 404);
  ptyService.removeInstance(id);
  instances.delete(id);
  return c.json({ success: true });
});
