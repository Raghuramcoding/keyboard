import { state } from './state.js';
import { TitleBar } from './components/title-bar.js';
import { Sidebar } from './components/sidebar.js';
import { RightPanel } from './components/right-panel.js';
import { SettingsModal } from './components/settings-modal.js';
import { InstanceDashboard } from './components/instance-dashboard.js';

export function initApp(): void {
    state.loadInstances();
    renderApp();

    window.addEventListener('hashchange', () => {
        renderApp();
    });
}

function renderApp(): void {
    const root = document.getElementById('app-root');
    if (!root) return;

    const hash = window.location.hash;
    const showLanding = !hash || hash === '#';

    if (showLanding) {
        root.innerHTML = `
            <div class="landing-container">
                <nav class="nav-pill">
                    <span>T.C.K</span>
                    <a href="#app" style="color: var(--fg); text-decoration: none; font-weight: 600;">Launch App →</a>
                </nav>
                <div class="hero">
                    <div class="hero-badge">Unified AI Environment</div>
                    <h1 class="hero-title">The Keyboard for<br>Modern Intelligence</h1>
                    <p class="hero-sub">A high-performance, unified TypeScript environment for AI-driven engineering. Terminal, Editor, and Orchestration in one seamless flow.</p>
                    <a href="#app" class="cta-primary">Launch T.C.K</a>
                </div>
                <div class="feature-grid">
                    <div class="feature-card">
                        <div class="feature-icon">⌨</div>
                        <h3 class="feature-title">Native PTY</h3>
                        <p class="feature-desc">Real-time shell access via node-pty, streamed over high-speed WebSockets.</p>
                    </div>
                    <div class="feature-card">
                        <div class="feature-icon">⚡</div>
                        <h3 class="feature-title">Unified AI</h3>
                        <p class="feature-desc">Direct integration with Claude, OpenAI, and Ollama via a unified Hono proxy.</p>
                    </div>
                    <div class="feature-card">
                        <div class="feature-icon">⚙</div>
                        <h3 class="feature-title">Zero Slop</h3>
                        <p class="feature-desc">Built with Ponytail principles: minimal dependencies, maximum efficiency.</p>
                    </div>
                </div>
                <footer class="footer">
                    <p>© 2026 TalentCloud Keyboard. Built for the technical elite.</p>
                </footer>
            </div>
        `;
        return;
    }

    root.innerHTML = `
        <div class="app">
            <div class="accent-bar"></div>
            <div id="titlebar"></div>
            <div class="body">
                <div id="sidebar"></div>
                <div class="main">
                    <div id="instance-dashboard"></div>
                </div>
                <div id="right-panel"></div>
            </div>
            <div id="settings-modal"></div>
        </div>
    `;

    const titlebarEl = document.getElementById('titlebar');
    const sidebarEl = document.getElementById('sidebar');
    const dashboardEl = document.getElementById('instance-dashboard');
    const rightPanelEl = document.getElementById('right-panel');
    const settingsEl = document.getElementById('settings-modal');

    if (titlebarEl) new TitleBar(state, titlebarEl).render();
    if (sidebarEl) new Sidebar(state, sidebarEl).render();
    if (dashboardEl) new InstanceDashboard(state, dashboardEl).render();
    if (rightPanelEl) new RightPanel(state, rightPanelEl).render();
    if (settingsEl) new SettingsModal(state, settingsEl).render();
}

initApp();
