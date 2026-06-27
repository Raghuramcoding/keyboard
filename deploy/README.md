# Deploying T.C.K to your own server (Hetzner, etc.)

T.C.K's backend is a long-running Rust process (it holds the terminal WebSocket
open, spawns a real shell, and keeps OAuth sessions in memory), so it wants a
**real server/VM** — a Hetzner Cloud instance is a great fit. (It will *not* run
on serverless platforms like Vercel; only the static UI can go there.)

> ⚠️ **Read the security note first.** The terminal is a real shell with the
> server process's privileges, and the app has **no built-in authentication**.
> Anyone who can reach an open T.C.K instance gets a shell on your box. Always put
> it behind **basic auth and/or a firewall/VPN**, and run it as a non-root user.

## Prerequisites

- A Linux VM (Hetzner Cloud, Ubuntu 22.04/24.04 or Debian 12 is fine).
- A domain with an **A record pointing at the server's IP** (needed for HTTPS).
- Inbound **ports 80 and 443** open in the Hetzner firewall.

---

## Option A — Docker + Caddy (recommended)

Caddy gets a free Let's Encrypt cert automatically and reverse-proxies to the app
behind HTTP basic auth.

```bash
# 1. Install Docker
curl -fsSL https://get.docker.com | sh

# 2. Get the code
git clone https://github.com/Raghuramcoding/keyboard.git && cd keyboard

# 3. Configure
cp deploy/.env.example .env
#   - set TCK_DOMAIN to your domain
#   - (optional) fill the TCK_GITHUB_* values for the Git panel

# 4. Set the basic-auth password: generate a hash and paste it into deploy/Caddyfile
docker run --rm caddy caddy hash-password --plaintext 'your-strong-password'
#   -> replace the "admin $2a$14$REPLACE..." line in deploy/Caddyfile

# 5. Build + run (first build compiles Rust + WASM, ~10 min)
docker compose up -d --build
```

Open `https://your-domain/`, enter the basic-auth credentials, and you're in.

**Update later:**

```bash
git pull && docker compose up -d --build
```

---

## Option B — systemd + reverse proxy (no Docker)

Build natively and run the binary under systemd, with Caddy or nginx in front.

```bash
# Build (on the server, or build elsewhere and copy the artifacts)
rustup target add wasm32-unknown-unknown
cargo install trunk
( cd crates/tck-ui && trunk build --release )
cargo build -p tck-server --release

# Install
sudo useradd -m tck
sudo mkdir -p /opt/tck/dist
sudo cp target/release/tck-server /opt/tck/
sudo cp -r crates/tck-ui/dist/* /opt/tck/dist/
sudo chown -R tck:tck /opt/tck

# Secrets (optional GitHub OAuth)
sudo tee /etc/tck.env >/dev/null <<'EOF'
TCK_GITHUB_CLIENT_ID=
TCK_GITHUB_CLIENT_SECRET=
TCK_GITHUB_CALLBACK=https://your-domain/auth/github/callback
EOF
sudo chmod 600 /etc/tck.env

# Service (binds 127.0.0.1:3000)
sudo cp deploy/tck-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now tck-server
```

Then point a TLS reverse proxy at `127.0.0.1:3000`. With Caddy it's a one-liner
`/etc/caddy/Caddyfile`:

```
your-domain {
    basic_auth {
        admin <PASTE_HASH_FROM_caddy_hash-password>
    }
    reverse_proxy 127.0.0.1:3000
}
```

---

## GitHub OAuth (Git / Source Control panel)

1. Create an **OAuth App** at <https://github.com/settings/developers> →
   *New OAuth App*:
   - **Homepage URL:** `https://your-domain`
   - **Authorization callback URL:** `https://your-domain/auth/github/callback`
2. Put the **Client ID / secret** into `.env` (Docker) or `/etc/tck.env` (systemd),
   and set `TCK_GITHUB_CALLBACK=https://your-domain/auth/github/callback`.
3. Restart, then click **🔗 Connect GitHub** in the app.

Because it's served over HTTPS, the terminal's WebSocket uses `wss://` and the
OAuth flow works end-to-end.

## Hardening checklist

- ✅ Keep **basic auth** on (or restrict the port to a VPN/your IP via the Hetzner
  firewall).
- ✅ The container/systemd unit runs as a **non-root** user (`tck`).
- ✅ Consider running on a **throwaway VM** if you let others reach the terminal.
- ✅ Rotate the basic-auth password and the OAuth client secret if leaked.
