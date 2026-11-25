import { HttpError } from './lib/errors.js';
import { synthesizeSpeech } from './providers/index.js';

// Limits what one request can cost. Engines also have their own limits.
const MAX_BODY_BYTES = 20000;

const ALLOWED_ORIGINS = (process.env.ALLOWED_ORIGINS ||
  'https://daisy.github.io,http://localhost:8080,http://127.0.0.1:8080')
  .split(',')
  .map((origin) => origin.trim())
  .filter(Boolean);

function getHeader(event, name) {
  const headers = event.headers || {};
  const wanted = name.toLowerCase();
  for (const [key, value] of Object.entries(headers)) {
    if (key.toLowerCase() === wanted) {
      return Array.isArray(value) ? (value[0] || '') : (value || '');
    }
  }
  return '';
}

function isAllowedOrigin(origin) {
  if (!origin) {
    return false;
  }
  if (ALLOWED_ORIGINS.includes(origin)) {
    return true;
  }
  try {
    const url = new URL(origin);
    return (url.hostname === 'localhost' || url.hostname === '127.0.0.1' || url.hostname === '::1') &&
      (url.protocol === 'http:' || url.protocol === 'https:');
  } catch {
    return false;
  }
}

function corsHeaders(origin) {
  const headers = {
    'Access-Control-Allow-Methods': 'POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
  };
  if (origin) {
    headers['Access-Control-Allow-Origin'] = origin;
  }
  return headers;
}

function resolveOrigin(event) {
  const origin = getHeader(event, 'origin');
  return isAllowedOrigin(origin) ? origin : '';
}

function jsonResponse(statusCode, origin, body) {
  return {
    statusCode,
    headers: { ...corsHeaders(origin), 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  };
}

export const handler = async (event) => {
  const origin = resolveOrigin(event);
  const method = event.requestContext?.http?.method || event.httpMethod || 'GET';

  if (method === 'OPTIONS') {
    return {
      statusCode: 204,
      headers: corsHeaders(origin),
      body: '',
    };
  }

  if (method !== 'POST') {
    return jsonResponse(405, origin, { error: 'Method not allowed' });
  }

  // CORS headers only hide the response; refuse here so nothing is billed.
  if (!origin) {
    return jsonResponse(403, origin, { error: 'Origin not allowed' });
  }

  if (Buffer.byteLength(event.body || '', 'utf8') > MAX_BODY_BYTES) {
    return jsonResponse(413, origin, { error: 'Request is too large' });
  }

  let body;
  try {
    body = JSON.parse(event.body || '{}');
  } catch {
    return jsonResponse(400, origin, { error: 'Invalid JSON body' });
  }

  const { text, lang } = body || {};
  if (!text || typeof text !== 'string') {
    return jsonResponse(400, origin, { error: 'text is required' });
  }
  if (lang && typeof lang !== 'string') {
    return jsonResponse(400, origin, { error: 'lang must be a string' });
  }

  try {
    const result = await synthesizeSpeech({
      text,
      lang: lang || 'en',
    });
    return jsonResponse(200, origin, result);
  } catch (err) {
    if (err instanceof HttpError) {
      return jsonResponse(err.statusCode, origin, { error: err.message });
    }
    console.error('TTS synthesis failed:', err);
    return jsonResponse(502, origin, { error: 'TTS synthesis failed' });
  }
};
