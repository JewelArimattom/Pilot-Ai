#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

function parseEnvFile(filePath) {
  if (!fs.existsSync(filePath)) {
    return {};
  }
  const raw = fs.readFileSync(filePath, "utf8");
  const lines = raw.split(/\r?\n/);
  const env = {};
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }
    const index = trimmed.indexOf("=");
    if (index === -1) {
      continue;
    }
    const key = trimmed.slice(0, index).trim();
    let value = trimmed.slice(index + 1).trim();
    if (
      (value.startsWith("\"") && value.endsWith("\"")) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    env[key] = value;
  }
  return env;
}

function getEnvValue(envFile, name, fallback) {
  if (envFile[name] !== undefined) {
    return envFile[name];
  }
  if (process.env[name] !== undefined) {
    return process.env[name];
  }
  return fallback;
}

function parsePositiveInt(value, fallback) {
  const parsed = Number.parseInt(String(value), 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function maskKey(key) {
  if (!key || key.length < 10) {
    return "REDACTED";
  }
  return `${key.slice(0, 4)}...${key.slice(-4)}`;
}

function truncate(text, max = 300) {
  if (!text) {
    return "";
  }
  return text.length > max ? `${text.slice(0, max)}...` : text;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function testKey({ key, index, endpointBase, payload }) {
  const url = `${endpointBase}${encodeURIComponent(key)}`;
  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify(payload)
  });

  if (response.ok) {
    return {
      ok: true,
      status: response.status,
      message: "OK"
    };
  }

  let message = response.statusText || "Request failed";
  try {
    const data = await response.json();
    message = data?.error?.message || message;
  } catch {
    try {
      const text = await response.text();
      if (text.trim()) {
        message = text.trim();
      }
    } catch {
      // Ignore parse errors.
    }
  }

  return {
    ok: false,
    status: response.status,
    message: truncate(message)
  };
}

async function main() {
  if (typeof fetch !== "function") {
    console.error("This script requires Node.js 18+ (global fetch support).");
    process.exit(1);
  }

  const envPath = path.resolve(process.cwd(), ".env");
  const envFile = parseEnvFile(envPath);

  const rawKeys = getEnvValue(envFile, "GEMINI_API_KEYS", "");
  if (!rawKeys) {
    console.error("No GEMINI_API_KEYS found in .env or process environment.");
    process.exit(1);
  }

  const keys = rawKeys
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);

  if (keys.length === 0) {
    console.error("GEMINI_API_KEYS is empty after parsing.");
    process.exit(1);
  }

  const model = getEnvValue(envFile, "GEMINI_MODEL", "gemini-1.5-flash");
  const apiVersion = getEnvValue(envFile, "GEMINI_API_VERSION", "v1");
  const maxOutputTokens = parsePositiveInt(
    getEnvValue(envFile, "GEMINI_MAX_OUTPUT_TOKENS", "32"),
    32
  );

  const endpointBase = `https://generativelanguage.googleapis.com/${apiVersion}/models/${model}:generateContent?key=`;
  const payload = {
    contents: [
      {
        role: "user",
        parts: [{ text: "ping" }]
      }
    ],
    generationConfig: {
      temperature: 0.1,
      maxOutputTokens
    }
  };

  console.log(`Testing ${keys.length} Gemini key(s) against model ${model} (${apiVersion}).`);

  for (let i = 0; i < keys.length; i += 1) {
    const key = keys[i];
    const label = `Key ${i + 1} (${maskKey(key)})`;
    try {
      const result = await testKey({ key, index: i, endpointBase, payload });
      const status = result.ok ? "OK" : `FAIL ${result.status}`;
      console.log(`${label}: ${status} - ${result.message}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.log(`${label}: FAIL - ${truncate(message)}`);
    }
    await sleep(250);
  }
}

main().catch((error) => {
  console.error("Unexpected error:", error);
  process.exit(1);
});
