const sharp = require('sharp');
const fs = require('fs');

async function generateIcon(svg, filename) {
  await sharp(Buffer.from(svg), { density: 300 })
    .resize(1024, 1024)
    .png()
    .toFile(filename);
  console.log('Generated:', filename);
}

// ========================================
// Style 1: Gradient Flow - Blue-purple gradient + white X flow lines
// ========================================
const style1 = `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#6366F1"/>
      <stop offset="50%" style="stop-color:#8B5CF6"/>
      <stop offset="100%" style="stop-color:#A855F7"/>
    </linearGradient>
    <linearGradient id="flow1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#FFFFFF;stop-opacity:0.95"/>
      <stop offset="100%" style="stop-color:#E0E7FF;stop-opacity:0.85"/>
    </linearGradient>
    <filter id="shadow1">
      <feDropShadow dx="0" dy="8" stdDeviation="24" flood-color="#4C1D95" flood-opacity="0.3"/>
    </filter>
  </defs>
  <rect x="64" y="64" width="896" height="896" rx="196" ry="196" fill="url(#bg1)" filter="url(#shadow1)"/>
  <g transform="translate(512,512)">
    <path d="M-220,-180 C-120,-80 -60,-20 0,40 C60,100 120,160 220,240"
          fill="none" stroke="url(#flow1)" stroke-width="72" stroke-linecap="round" opacity="0.9"/>
    <path d="M220,-180 C120,-80 60,-20 0,40 C-60,100 -120,160 -220,240"
          fill="none" stroke="url(#flow1)" stroke-width="72" stroke-linecap="round" opacity="0.9"/>
    <circle cx="0" cy="40" r="48" fill="#FFFFFF" opacity="0.95"/>
    <circle cx="-140" cy="-80" r="18" fill="#C4B5FD" opacity="0.8"/>
    <circle cx="140" cy="160" r="14" fill="#DDD6FE" opacity="0.7"/>
    <circle cx="-100" cy="180" r="12" fill="#EDE9FE" opacity="0.6"/>
  </g>
</svg>`;

// ========================================
// Style 2: AI Tech - Dark background + glowing AI star
// ========================================
const style2 = `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#0F172A"/>
      <stop offset="100%" style="stop-color:#1E293B"/>
    </linearGradient>
    <linearGradient id="star2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#22D3EE"/>
      <stop offset="50%" style="stop-color:#60A5FA"/>
      <stop offset="100%" style="stop-color:#A78BFA"/>
    </linearGradient>
    <filter id="glow2">
      <feGaussianBlur stdDeviation="12" result="blur"/>
      <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
    <filter id="glowStrong2">
      <feGaussianBlur stdDeviation="24" result="blur"/>
      <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>
  <rect x="0" y="0" width="1024" height="1024" fill="url(#bg2)"/>
  <g transform="translate(512,480)">
    <circle cx="0" cy="0" r="180" fill="none" stroke="url(#star2)" stroke-width="6" opacity="0.3" filter="url(#glow2)"/>
    <circle cx="0" cy="0" r="140" fill="none" stroke="url(#star2)" stroke-width="4" opacity="0.5"/>
    <polygon points="0,-160 35,-80 120,-60 60,0 120,60 35,80 0,160 -35,80 -120,60 -60,0 -120,-60 -35,-80"
             fill="none" stroke="url(#star2)" stroke-width="10" stroke-linejoin="round"
             filter="url(#glowStrong2)"/>
    <polygon points="0,-100 22,-50 75,-38 38,0 75,38 22,50 0,100 -22,50 -75,38 -38,0 -75,-38 -22,-50"
             fill="url(#star2)" opacity="0.8" filter="url(#glow2)"/>
    <circle cx="0" cy="0" r="24" fill="#FFFFFF" filter="url(#glowStrong2)"/>
    <circle cx="190" cy="0" r="8" fill="#22D3EE" opacity="0.8" filter="url(#glow2)"/>
    <circle cx="-190" cy="0" r="8" fill="#A78BFA" opacity="0.8" filter="url(#glow2)"/>
    <circle cx="0" cy="190" r="6" fill="#60A5FA" opacity="0.6" filter="url(#glow2)"/>
    <circle cx="0" cy="-190" r="6" fill="#60A5FA" opacity="0.6" filter="url(#glow2)"/>
  </g>
  <text x="512" y="820" text-anchor="middle" font-family="Arial, sans-serif" font-size="48" font-weight="bold"
        fill="#ffffff" opacity="0.15" letter-spacing="20">XUFLOW</text>
</svg>`;

// ========================================
// Style 3: Clean Badge - Like Notion/Linear style
// ========================================
const style3 = `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg3" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" style="stop-color:#EEF2FF"/>
      <stop offset="100%" style="stop-color:#E0E7FF"/>
    </linearGradient>
    <linearGradient id="icon3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#4F46E5"/>
      <stop offset="100%" style="stop-color:#7C3AED"/>
    </linearGradient>
    <filter id="shadow3">
      <feDropShadow dx="0" dy="4" stdDeviation="16" flood-color="#3730A3" flood-opacity="0.15"/>
    </filter>
  </defs>
  <rect x="0" y="0" width="1024" height="1024" fill="url(#bg3)"/>
  <circle cx="512" cy="480" r="280" fill="url(#icon3)" filter="url(#shadow3)"/>
  <circle cx="512" cy="480" r="280" fill="none" stroke="#FFFFFF" stroke-width="3" opacity="0.2"/>
  <circle cx="512" cy="480" r="260" fill="none" stroke="#FFFFFF" stroke-width="1.5" opacity="0.1"/>
  <g transform="translate(512,480)">
    <rect x="-160" y="-70" width="320" height="140" rx="70" ry="70"
          fill="#FFFFFF" transform="rotate(45)"/>
    <rect x="-160" y="-70" width="320" height="140" rx="70" ry="70"
          fill="#FFFFFF" transform="rotate(-45)"/>
    <circle cx="0" cy="0" r="36" fill="#4F46E5" opacity="0.3"/>
  </g>
  <text x="512" y="850" text-anchor="middle" font-family="Arial, sans-serif" font-size="72" font-weight="700"
        fill="#312E81" letter-spacing="4">XUFLOW</text>
</svg>`;

async function main() {
  const dir = 'D:/Projects-star/Xuflow-desktop/icon-examples';
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

  await generateIcon(style1, dir + '/style1-gradient-flow.png');
  await generateIcon(style2, dir + '/style2-ai-tech.png');
  await generateIcon(style3, dir + '/style3-clean-badge.png');
  console.log('Done! All 3 icon examples generated in icon-examples/');
}

main().catch(console.error);
