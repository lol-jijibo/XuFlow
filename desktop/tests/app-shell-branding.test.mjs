import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

function readPngSize(path) {
  const bytes = readFileSync(path);
  assert.equal(bytes.toString("ascii", 1, 4), "PNG", `${path} must be a PNG`);

  return {
    width: bytes.readUInt32BE(16),
    height: bytes.readUInt32BE(20),
  };
}

function readIcoEntries(path) {
  const bytes = readFileSync(path);
  assert.equal(bytes.readUInt16LE(0), 0, `${path} must start with an ICO reserved field`);
  assert.equal(bytes.readUInt16LE(2), 1, `${path} must be an ICO file`);

  const count = bytes.readUInt16LE(4);
  return Array.from({ length: count }, (_, index) => {
    const offset = 6 + index * 16;
    const width = bytes[offset] === 0 ? 256 : bytes[offset];
    const height = bytes[offset + 1] === 0 ? 256 : bytes[offset + 1];

    return { width, height };
  });
}

test("Tauri shell exposes the XuFlow brand in Windows taskbar and tray hover text", () => {
  const config = JSON.parse(
    readFileSync(resolve("desktop/src-tauri/tauri.conf.json"), "utf8")
  );

  assert.equal(config.productName, "XuFlow");
  assert.equal(config.app.windows[0].title, "XuFlow");
  assert.equal(config.app.trayIcon.tooltip, "XuFlow");
  assert.equal(config.app.trayIcon.title, "XuFlow");
});

test("Tauri icon resources use valid Windows taskbar and tray dimensions", () => {
  const iconDir = resolve("desktop/src-tauri/icons");

  assert.deepEqual(readPngSize(resolve(iconDir, "16x16.png")), {
    width: 16,
    height: 16,
  });
  assert.deepEqual(readPngSize(resolve(iconDir, "20x20.png")), {
    width: 20,
    height: 20,
  });
  assert.deepEqual(readPngSize(resolve(iconDir, "24x24.png")), {
    width: 24,
    height: 24,
  });
  assert.deepEqual(readPngSize(resolve(iconDir, "32x32.png")), {
    width: 32,
    height: 32,
  });
  assert.deepEqual(readPngSize(resolve(iconDir, "40x40.png")), {
    width: 40,
    height: 40,
  });
  assert.deepEqual(readPngSize(resolve(iconDir, "48x48.png")), {
    width: 48,
    height: 48,
  });
  assert.deepEqual(readPngSize(resolve(iconDir, "64x64.png")), {
    width: 64,
    height: 64,
  });
  assert.deepEqual(readPngSize(resolve(iconDir, "128x128.png")), {
    width: 128,
    height: 128,
  });
  assert.deepEqual(readPngSize(resolve(iconDir, "128x128@2x.png")), {
    width: 256,
    height: 256,
  });
  assert.deepEqual(readPngSize(resolve(iconDir, "icon.png")), {
    width: 256,
    height: 256,
  });
  assert.deepEqual(readIcoEntries(resolve(iconDir, "icon.ico")), [
    { width: 16, height: 16 },
    { width: 20, height: 20 },
    { width: 24, height: 24 },
    { width: 32, height: 32 },
    { width: 40, height: 40 },
    { width: 48, height: 48 },
    { width: 64, height: 64 },
    { width: 128, height: 128 },
    { width: 256, height: 256 },
  ]);
});
