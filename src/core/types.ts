export interface Agent { id: string; label: string; command: string; icon: string; }
export interface AgentRole { id: string; name: string; icon: string; system_prompt: string; }
export interface PlannedTask { role: string; title: string; instructions: string; }
export enum Role { User, Ai, Err }
export interface ChatMsg { role: Role; text: string; }
export interface CatalogModel { id: string; provider: string; name: string; context_window: number; supports_tools: boolean; supports_streaming: boolean; }
export interface ProviderConfig { id: string; name: string; base_url: string; api_key: string; models: string[]; }
export interface CustomCommand { id: string; label: string; command: string; icon: string; }
export interface Settings { providers: ProviderConfig[]; commands: CustomCommand[]; claude_api_key: string; ollama_host: string; background: string; }
export interface GenerateRequest { provider: string; model: string; prompt: string; system?: string; base_url?: string; api_key?: string; temperature?: number; max_tokens?: number; }
export interface GenerateResponse { response?: string; error?: string; model?: string; }
export enum InstanceKind { Terminal, Chat }
export enum AgentState { Idle, Working, Done, Blocked, Error }
export enum InstanceStatus { Running, Stopped, Error }
export interface InstanceInfo { id: string; kind: InstanceKind; label: string; command?: string; cwd?: string; status: InstanceStatus; error?: string; created_at: number; agent_state: AgentState; last_output_at: number; }
export type EventMessage =
  | { type: 'agent_state_changed'; instance_id: string; old_state: AgentState; new_state: AgentState; hint: string; timestamp: number; }
  | { type: 'instance_status_changed'; instance_id: string; status: InstanceStatus; timestamp: number; }
  | { type: 'output_activity'; instance_id: string; bytes_written: number; timestamp: number; }
  | { type: 'heartbeat'; timestamp: number; };
