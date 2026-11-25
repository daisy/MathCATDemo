import { HttpError } from '../lib/errors.js';
import { isTooLong, prepareGoogleSsml, restoreMarkNames } from '../lib/google-ssml.js';
import { googleMarksFromTimepoints } from '../lib/marks.js';
import { localeFromVoice } from '../lib/voices.js';

// Timepoints (needed for sync highlighting) exist only in v1beta1.
const ENDPOINT = 'https://texttospeech.googleapis.com/v1beta1/text:synthesize';
const TIMEOUT_MS = 15000;

export async function synthesize({ ssml, voice }) {
  const apiKey = process.env.GOOGLE_TTS_API_KEY;
  if (!apiKey) {
    throw new Error('GOOGLE_TTS_API_KEY is not configured');
  }

  const languageCode = localeFromVoice(voice);
  const prepared = prepareGoogleSsml(ssml, languageCode);

  const response = await fetch(ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Goog-Api-Key': apiKey,
    },
    body: JSON.stringify({
      input: { ssml: prepared.ssml },
      voice: { languageCode, name: voice },
      audioConfig: { audioEncoding: 'MP3' },
      enableTimePointing: ['SSML_MARK'],
    }),
    signal: AbortSignal.timeout(TIMEOUT_MS),
  });

  if (!response.ok) {
    const details = `Google TTS error ${response.status}: ${await response.text()}`;
    if (response.status === 400 && isTooLong(prepared.ssml)) {
      console.warn(details);
      throw new HttpError(413, 'Expression is too long to speak');
    }
    throw new Error(details);
  }

  const { audioContent, timepoints } = await response.json();
  if (!audioContent) {
    throw new Error('Google TTS returned no audio');
  }

  return {
    marks: googleMarksFromTimepoints(restoreMarkNames(timepoints, prepared.names)),
    audioBase64: audioContent,
    mimeType: 'audio/mpeg',
  };
}
