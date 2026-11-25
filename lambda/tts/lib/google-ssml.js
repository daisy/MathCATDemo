// Google's limit on the size of one request. In testing, marks did not count toward it.
// https://docs.cloud.google.com/text-to-speech/quotas
const GOOGLE_MAX_BYTES = 5000;

const MARK = /<mark\s+name=(['"])(.*?)\1\s*\/>/gi;

// Google may drop events for marks that have no audio between them, so keep only
// the last mark of each run. These tags produce no audio by themselves.
// <break> and <audio> do, so they end a run.
const SUPERSEDED_MARK =
  /<mark\s[^>]*\/>(?=(?:\s|<\/?(?:prosody|say-as|phoneme|emphasis|sub|voice|p|s)\b[^>]*>)*<mark\s)/gi;

// Google drops a mark that is separated from a following tag by whitespace, so move
// the whitespace in front of the mark.
const MARK_SPACE_TAG = /\s*(<mark\s[^>]*\/>)\s+(?=<)/gi;

const IPA_PHONEME = /<phoneme\s[^>]*alphabet=(['"])ipa\1[^>]*>([\s\S]*?)<\/phoneme>/gi;

const ENTITIES = { '&lt;': '<', '&gt;': '>', '&quot;': '"', '&apos;': "'", '&amp;': '&' };

function unescapeXml(text) {
  return text.replace(/&(?:lt|gt|quot|apos|amp);/g, (entity) => ENTITIES[entity]);
}

// Google accepts only pinyin/jyutping phonemes for Chinese and yomigana for Japanese.
function supportsIpa(locale) {
  return !/^(?:cmn|yue|ja)-/i.test(locale);
}

// Returns the SSML to send, plus `names`, where names[i] is the original mark name
// that was shortened to `m<i>` to save bytes.
export function prepareGoogleSsml(ssml, locale) {
  const names = [];
  const prepared = (supportsIpa(locale) ? ssml : ssml.replace(IPA_PHONEME, '$2'))
    .replace(SUPERSEDED_MARK, '')
    .replace(MARK_SPACE_TAG, ' $1')
    .replace(/^<speak>\s+/, '<speak>')
    .replace(MARK, (_match, _quote, name) => {
      names.push(unescapeXml(name));
      return `<mark name='m${names.length - 1}'/>`;
    });
  return { ssml: prepared, names };
}

export function restoreMarkNames(timepoints, names) {
  return (timepoints || []).map((point) => ({
    ...point,
    markName: names[Number(point.markName.slice(1))],
  }));
}

export function isTooLong(ssml) {
  return Buffer.byteLength(ssml.replace(MARK, ''), 'utf8') > GOOGLE_MAX_BYTES;
}
