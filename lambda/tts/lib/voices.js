const VOICES = {
  polly: {
    da: 'Naja',
    de: 'Vicki',
    en: 'Joanna',
    es: 'Lucia',
    fi: 'Suvi',
    fr: 'Lea',
    is: 'Dora',
    it: 'Bianca',
    nb: 'Ida',
    nl: 'Laura',
    pl: 'Ola',
    pt: 'Vitoria',
    sv: 'Astrid',
    'zh-cn': 'Hiujin',
    'zh-tw': 'Zhiyu',
  },
  google: {
    // https://docs.cloud.google.com/text-to-speech/docs/list-voices-and-types
    da: 'da-DK-Wavenet-F',
    de: 'de-DE-Wavenet-G',
    el: 'el-GR-Wavenet-B',
    en: 'en-US-Wavenet-F',
    'en-gb': 'en-GB-Wavenet-A',
    es: 'es-ES-Wavenet-F',
    fi: 'fi-FI-Wavenet-B',
    fr: 'fr-FR-Wavenet-F',
    hu: 'hu-HU-Wavenet-B',
    id: 'id-ID-Wavenet-A',
    is: 'is-IS-Standard-B', // Icelandic has no WaveNet voice
    it: 'it-IT-Wavenet-E',
    ja: 'ja-JP-Wavenet-A',
    nb: 'nb-NO-Wavenet-F',
    nl: 'nl-NL-Wavenet-F',
    pl: 'pl-PL-Wavenet-F',
    pt: 'pt-PT-Wavenet-E',
    ru: 'ru-RU-Wavenet-A',
    sv: 'sv-SE-Wavenet-A',
    vi: 'vi-VN-Wavenet-A',
    zh: 'cmn-CN-Wavenet-A',
    'zh-cn': 'cmn-CN-Wavenet-A',
    'zh-tw': 'cmn-TW-Wavenet-A',
  },
  azure: {
    da: 'da-DK-ChristelNeural',
    de: 'de-DE-KatjaNeural',
    en: 'en-US-JennyNeural',
    es: 'es-ES-ElviraNeural',
    fi: 'fi-FI-SelmaNeural',
    fr: 'fr-FR-DeniseNeural',
    id: 'id-ID-GadisNeural',
    is: 'is-IS-GudrunNeural',
    it: 'it-IT-ElsaNeural',
    nb: 'nb-NO-IselinNeural',
    nl: 'nl-NL-ColetteNeural',
    pl: 'pl-PL-AgnieszkaNeural',
    pt: 'pt-PT-RaquelNeural',
    sv: 'sv-SE-SofieNeural',
    vi: 'vi-VN-HoaiMyNeural',
    'zh-cn': 'zh-CN-XiaoxiaoNeural',
    'zh-tw': 'zh-TW-HsiaoChenNeural',
  },
};

// Tries the exact language, then its base ("pt-br" -> "pt"), then English.
export function voiceForLang(provider, lang) {
  const table = VOICES[provider] || VOICES.polly;
  const key = lang?.toLowerCase() || 'en';
  return table[key] || table[key.split('-')[0]] || table.en;
}

// "cmn-TW-Wavenet-A" -> "cmn-TW". Google and Azure voice names start with their locale.
export function localeFromVoice(voice) {
  return voice.split('-').slice(0, 2).join('-');
}
