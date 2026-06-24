// Generates the app icon (build/icon.png / .ico / .icns) by rendering an SVG
// offscreen in Electron and converting with png2icons (pure JS, no native deps).
// Run: npx electron gen-icon.js
const { app, BrowserWindow } = require('electron');
const fs = require('fs');
const path = require('path');
const png2icons = require('png2icons');

const SIZE = 1024;

// "Keyboard" mark: a clean white keyboard on a blue gradient badge.
function svg() {
  const keyW = 96, keyH = 84, gap = 26, rx = 18;
  const cols = 6;
  const gridW = cols * keyW + (cols - 1) * gap; // 6*96 + 5*26 = 706
  const startX = (SIZE - gridW) / 2;             // centered
  const startY = 372;
  const keys = [];
  // three rows of keys
  for (let r = 0; r < 3; r++) {
    const y = startY + r * (keyH + gap);
    const n = r === 1 ? cols - 1 : cols; // middle row slightly indented
    const offX = r === 1 ? startX + (keyW + gap) / 2 : startX;
    for (let c = 0; c < n; c++) {
      keys.push(`<rect x="${offX + c * (keyW + gap)}" y="${y}" width="${keyW}" height="${keyH}" rx="${rx}" fill="#ffffff"/>`);
    }
  }
  // spacebar
  const sbY = startY + 3 * (keyH + gap);
  const sbX = startX + keyW + gap;
  const sbW = gridW - 2 * (keyW + gap);
  keys.push(`<rect x="${sbX}" y="${sbY}" width="${sbW}" height="${keyH}" rx="${rx}" fill="#ffffff"/>`);

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}">
    <defs>
      <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#1f9cf0"/>
        <stop offset="0.55" stop-color="#0e639c"/>
        <stop offset="1" stop-color="#0a4a76"/>
      </linearGradient>
      <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%">
        <feDropShadow dx="0" dy="14" stdDeviation="18" flood-color="#000000" flood-opacity="0.28"/>
      </filter>
    </defs>
    <rect width="${SIZE}" height="${SIZE}" rx="224" fill="url(#bg)"/>
    <g filter="url(#shadow)" opacity="0.97">${keys.join('')}</g>
  </svg>`;
}

app.whenReady().then(async () => {
  const win = new BrowserWindow({ width: SIZE, height: SIZE, show: false, frame: false });
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>html,body{margin:0;padding:0;overflow:hidden}svg{display:block}</style></head><body>${svg()}</body></html>`;
  await win.loadURL('data:text/html;charset=utf-8,' + encodeURIComponent(html));
  await new Promise((r) => setTimeout(r, 400));
  const img = (await win.webContents.capturePage()).resize({ width: SIZE, height: SIZE });
  const png = img.toPNG();

  const outDir = path.join(__dirname, 'build');
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(path.join(outDir, 'icon.png'), png);
  fs.writeFileSync(path.join(outDir, 'icon.ico'), png2icons.createICO(png, png2icons.BILINEAR, 0, false));
  fs.writeFileSync(path.join(outDir, 'icon.icns'), png2icons.createICNS(png, png2icons.BILINEAR, 0));

  console.log('wrote build/icon.png (' + png.length + 'b), icon.ico, icon.icns');
  app.exit(0);
});
