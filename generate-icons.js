const fs = require("fs");
const path = require("path");
const sharp = require("sharp");

const rootDir = __dirname;
const sourceIcon = path.join(rootDir, "desktop", "public", "xuflow.png");
const iconDir = path.join(rootDir, "desktop", "src-tauri", "icons");
const pngSizes = [16, 20, 24, 32, 40, 48, 64, 128, 256];
const icoSizes = [16, 20, 24, 32, 40, 48, 64, 128, 256];

async function renderPng(size) {
  const output = path.join(iconDir, `${size}x${size}.png`);

  await sharp(sourceIcon)
    .resize(size, size, {
      fit: "cover",
      kernel: size <= 48 ? sharp.kernel.lanczos3 : sharp.kernel.mks2021,
    })
    .sharpen(size <= 48 ? { sigma: 0.65, m1: 1.2, m2: 1.9 } : { sigma: 0.35 })
    .png({ compressionLevel: 9, adaptiveFiltering: true })
    .toFile(output);

  return output;
}

function createIcoEntry(png, index) {
  const headerSize = 6 + icoSizes.length * 16;
  const priorSize = pngsForIco
    .slice(0, index)
    .reduce((total, current) => total + current.length, 0);
  const imageOffset = headerSize + priorSize;
  const size = icoSizes[index];
  const entry = Buffer.alloc(16);

  entry.writeUInt8(size === 256 ? 0 : size, 0);
  entry.writeUInt8(size === 256 ? 0 : size, 1);
  entry.writeUInt8(0, 2);
  entry.writeUInt8(0, 3);
  entry.writeUInt16LE(1, 4);
  entry.writeUInt16LE(32, 6);
  entry.writeUInt32LE(png.length, 8);
  entry.writeUInt32LE(imageOffset, 12);

  return entry;
}

let pngsForIco = [];

async function writeIco() {
  pngsForIco = await Promise.all(
    icoSizes.map((size) => fs.promises.readFile(path.join(iconDir, `${size}x${size}.png`)))
  );

  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(icoSizes.length, 4);

  const entries = pngsForIco.map(createIcoEntry);
  await fs.promises.writeFile(path.join(iconDir, "icon.ico"), Buffer.concat([header, ...entries, ...pngsForIco]));
}

async function main() {
  await fs.promises.mkdir(iconDir, { recursive: true });
  await Promise.all(pngSizes.map(renderPng));
  await fs.promises.copyFile(path.join(iconDir, "256x256.png"), path.join(iconDir, "icon.png"));
  await fs.promises.copyFile(path.join(iconDir, "256x256.png"), path.join(iconDir, "128x128@2x.png"));
  await writeIco();
  console.log("Generated Tauri icons from desktop/public/xuflow.png");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
