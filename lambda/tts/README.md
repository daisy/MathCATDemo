# MathCAT Demo TTS Proxy

Serverless proxy for speech playback. The GitHub Pages frontend posts MathCAT SSML plus a language code; the proxy selects the configured engine and returns normalized speech marks and MP3 audio.

Supported engines (set with `TTS_PROVIDER`):

| Provider | Env vars |
|----------|----------|
| `polly` (default) | Uses the Lambda execution role (`polly:SynthesizeSpeech`) |
| `google` (WaveNet) | `GOOGLE_TTS_API_KEY` — API key restricted to the Text-to-Speech API |
| `azure` | `AZURE_SPEECH_KEY`, `AZURE_SPEECH_REGION` |

MathCAT emits W3C-style `<mark name="id"/>` bookmarks. Azure requires `<bookmark mark="id"/>`; the proxy converts that automatically when `TTS_PROVIDER=azure`.

For Google, the proxy merges marks that have no speech between them, moves whitespace that follows a mark in front of it (Google drops a mark separated from a following tag by whitespace), shortens mark names (restoring them in the response), and removes IPA `<phoneme>` tags for Chinese and Japanese, which Google does not accept. If Google rejects an expression over its 5,000-byte limit (marks do not appear to count), the proxy returns 413.

## Google setup

1. In a Google Cloud project with billing enabled, enable the **Cloud Text-to-Speech API**.
2. Create an API key and restrict it to the Text-to-Speech API.
3. Set a budget alert, and lower the API's requests-per-minute quota.
4. Add the key as the repository secret `GOOGLE_TTS_API_KEY`.
5. Test the key against every language (see below).
6. Set the repository secret `TTS_PROVIDER` to `google` and run the **Deploy TTS Lambda** workflow. To roll back, set it to `polly` and run the workflow again.

### Live test

Sends real MathCAT SSML to Google in every language, checks that every mark gets a time, and saves the MP3s. If it reaches Google's per-minute quota, it waits and retries. It makes about 60 billable requests and needs no `npm install`. In PowerShell:

```powershell
$env:GOOGLE_TTS_API_KEY = Read-Host "API key"
node test/live-google.mjs              # or: --lang en,zh-cn
Remove-Item Env:GOOGLE_TTS_API_KEY
```

## API

```http
POST /tts
Content-Type: application/json

{ "text": "<prosody>...</prosody>", "lang": "en" }
```

Response:

```json
{
  "marks": [{ "time": 120, "value": "element-id" }],
  "audioBase64": "...",
  "mimeType": "audio/mpeg"
}
```

## Deploy

```bash
cd lambda/tts
npm install
sam build
sam deploy --guided
```

Set `TtsProvider` during deploy (or in `samconfig.toml`). For Google/Azure, also pass the credential parameters or add them in the AWS Lambda console after deploy.

Note the `TtsApiUrl` output and set it in `index.html`:

```html
window.MATHCAT_TTS_API = 'https://YOUR-API-ID.execute-api.us-east-1.amazonaws.com/tts';
```

## GitHub Actions

Workflow `.github/workflows/deploy-tts-lambda.yml` deploys when `lambda/tts/**` changes.

Repository secrets for deploy:

- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `AWS_REGION`

The AWS user must be able to create and update the stack's resources: Lambda, API Gateway, IAM role, S3, and DynamoDB table (including time to live).

Optional secrets for non-Polly engines:

- `TTS_PROVIDER`
- `AZURE_SPEECH_KEY`
- `AZURE_SPEECH_REGION`
- `GOOGLE_TTS_API_KEY`

## Security

- Credentials stay on the server; the browser only sees the proxy URL.
- Restrict `AllowedOrigins` to your demo site and local dev URLs. Requests from other origins get 403. Non-browser callers can fake this header, so it is not authentication.
- **Daily cap:** at most `DailyCharLimit` characters (default 125,000) are sent to the engine per UTC day; after that, requests get 429 until the next day. The count is kept in the `UsageTable` DynamoDB table and covers all engines. To change the limit, set the repository **variable** `DAILY_CHAR_LIMIT` and run the deploy workflow; `0` turns speech off. Rejected or failed engine requests still count. The live test calls Google directly and is not counted.
- API Gateway throttles total traffic to slow abuse (`DefaultRouteSettings` in `template.yaml`).
- Enable billing alerts for whichever cloud TTS service you use.
