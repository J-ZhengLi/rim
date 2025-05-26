import { createHash } from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import process from 'node:process';
import { resolve, join } from 'node:path';
import { Buffer } from 'node:buffer';
import { fetch } from 'ofetch';

function replaceAsync(string, searchValue, replacer) {
  try {
    if (typeof replacer === "function") {
      const values = [];
      String.prototype.replace.call(string, searchValue, (...args) => {
        values.push(replacer(...args));
        return "";
      });
      return Promise.all(values).then((resolvedValues) => {
        return String.prototype.replace.call(string, searchValue, () => {
          return resolvedValues.shift() || "";
        });
      });
    } else {
      return Promise.resolve(
        String.prototype.replace.call(string, searchValue, replacer)
      );
    }
  } catch (error) {
    return Promise.reject(error);
  }
}

const fontUrlRegex = /[-\w@:%+.~#?&/=]+\.(?:woff2?|eot|ttf|otf|svg)/gi;
function createLocalFontProcessor(options) {
  const cwd = options?.cwd || process.cwd();
  const cacheDir = resolve(cwd, options?.cacheDir || "node_modules/.cache/unocss/fonts");
  const fontAssetsDir = resolve(cwd, options?.fontAssetsDir || "public/assets/fonts");
  const fontServeBaseUrl = options?.fontServeBaseUrl || "/assets/fonts";
  async function _downloadFont(url, assetPath) {
    const response = await fetch(url).then((r) => r.arrayBuffer());
    await fsp.mkdir(fontAssetsDir, { recursive: true });
    await fsp.writeFile(assetPath, Buffer.from(response));
  }
  const cache = /* @__PURE__ */ new Map();
  function downloadFont(url, assetPath) {
    if (!cache.has(url))
      cache.set(url, _downloadFont(url, assetPath));
    return cache.get(url);
  }
  return {
    async getCSS(fonts, providers, getCSSDefault) {
      const hash = getHash(JSON.stringify(fonts));
      const cachePath = join(cacheDir, `${hash}.css`);
      if (fs.existsSync(cachePath)) {
        return fsp.readFile(cachePath, "utf-8");
      }
      const css = await getCSSDefault(fonts, providers);
      await fsp.mkdir(cacheDir, { recursive: true });
      await fsp.writeFile(cachePath, css, "utf-8");
      return css;
    },
    async transformCSS(css) {
      return await replaceAsync(css, fontUrlRegex, async (url) => {
        const hash = getHash(url);
        const ext = url.split(".").pop();
        let name = "";
        const match1 = url.match(/\/s\/([^/]+)\//);
        if (match1)
          name = match1[1].replace(/\W/g, " ").trim().replace(/\s+/, "-").toLowerCase();
        const filename = `${[name, hash].filter(Boolean).join("-")}.${ext}`;
        const assetPath = join(fontAssetsDir, filename);
        if (!fs.existsSync(assetPath)) {
          await downloadFont(url, assetPath);
        }
        return `${fontServeBaseUrl}/${filename}`;
      });
    }
  };
}
function getHash(input, length = 8) {
  return createHash("sha256").update(input).digest("hex").slice(0, length);
}

export { createLocalFontProcessor };
