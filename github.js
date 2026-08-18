// Minimal GitHub Contents API client.
//
// Everything here runs in the browser against api.github.com, which does send
// CORS headers (the OAuth endpoints do not, which is why this uses a personal
// access token rather than a sign-in flow). See design.md section 3.

import { CONFIG } from "./config.js";

const API = "https://api.github.com";
const TOKEN_KEY = "leaners.token";

export function getToken() {
  return sessionStorage.getItem(TOKEN_KEY) || "";
}

export function setToken(token) {
  if (token) sessionStorage.setItem(TOKEN_KEY, token);
  else sessionStorage.removeItem(TOKEN_KEY);
}

export function repoCoords() {
  let { owner, repo, branch } = CONFIG;
  if (!owner || !repo) {
    const host = location.hostname.match(/^([^.]+)\.github\.io$/);
    const seg = location.pathname.split("/").filter(Boolean)[0];
    if (host) {
      owner ||= host[1];
      repo ||= seg || `${host[1]}.github.io`;
    }
  }
  return { owner, repo, branch: branch || "main" };
}

// btoa() cannot be used directly on text. Above U+00FF it throws, and between
// U+0080 and U+00FF it silently emits Latin-1, so "café" round-trips to "caf?".
// Going through UTF-8 bytes explicitly is what keeps accents and Lean's
// notation (forall, ->, lambda) intact.
function encodeB64(text) {
  const bytes = new TextEncoder().encode(text);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}

function decodeB64(b64) {
  const bin = atob(b64.replace(/\s/g, ""));
  return new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
}

async function api(path, options = {}) {
  const { owner, repo } = repoCoords();
  const token = getToken();
  if (!token) throw new Error("No token. Add one to edit.");

  const res = await fetch(`${API}/repos/${owner}/${repo}${path}`, {
    ...options,
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...options.headers,
    },
  });

  if (res.status === 401) throw new Error("Token rejected. Check it has Contents: read and write.");
  if (res.status === 403) throw new Error("Forbidden. The token may lack access to this repository.");
  if (res.status === 409) throw new Error("CONFLICT");
  if (!res.ok && res.status !== 404) {
    const detail = await res.json().catch(() => ({}));
    throw new Error(detail.message || `${res.status} ${res.statusText}`);
  }
  return res;
}

/** Returns {text, sha}, or {text: null, sha: null} if the file does not exist. */
export async function readFile(path) {
  const { branch } = repoCoords();
  const res = await api(`/contents/${path}?ref=${branch}`);
  if (res.status === 404) return { text: null, sha: null };
  const data = await res.json();
  return { text: decodeB64(data.content), sha: data.sha };
}

/** Creates or updates a file. Pass sha to update, omit it to create. */
export async function writeFile(path, text, message, sha) {
  const { branch } = repoCoords();
  const res = await api(`/contents/${path}`, {
    method: "PUT",
    body: JSON.stringify({
      message,
      content: encodeB64(text),
      branch,
      ...(sha ? { sha } : {}),
    }),
  });
  return res.json();
}

/** Confirms the token works and reports who it belongs to. */
export async function whoami() {
  const res = await fetch(`${API}/user`, {
    headers: { Authorization: `Bearer ${getToken()}`, Accept: "application/vnd.github+json" },
  });
  if (!res.ok) throw new Error("Token rejected.");
  return (await res.json()).login;
}
