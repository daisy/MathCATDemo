# MathCAT: Math Capable Assistive Technology Demo
<img src="logo.png" style="position: relative; top: 16px; z-index: -1;"> is a library that supports conversion of MathML to speech and braille among other things.
This project adds a GUI to MathCAT to demo some of its capabilities.
Visit [the MathCAT project page](https://daisy.github.io/MathCAT/) for more info or if you want to play around, [try out the demo](https://daisy.github.io/MathCATDemo/).


## Local builds
To build this and run locally, you need to download and install [trunk](https://docs.trunk.io/docs/install). Then type
```
trunk serve
```

## Text-to-speech

The demo can read SSML aloud with **sync highlighting** (speech marks tied to MathCAT element ids). That requires a cloud TTS engine, but credentials must not live in the static GitHub Pages site. A small **Lambda proxy** in [`lambda/tts`](lambda/tts/README.md) holds the secrets and synthesizes audio server-side.

The browser is engine-agnostic: it POSTs `{ text, lang }` to the proxy URL. Voice choice, SSML wrapping, and provider-specific fixes (e.g. Azure bookmark conversion) happen in the backend. Pick the engine with the `TTS_PROVIDER` Lambda environment variable.

| Provider | `TTS_PROVIDER` | Credentials (Lambda env / SAM parameters) |
|----------|----------------|---------------------------------------------|
| Amazon Polly (default) | `polly` | Lambda execution role (`polly:SynthesizeSpeech`) |
| Google Cloud TTS | `google` | `GOOGLE_SERVICE_ACCOUNT_JSON` (full service-account JSON) |
| Azure Speech | `azure` | `AZURE_SPEECH_KEY`, `AZURE_SPEECH_REGION` |

Note: MathCAT sets **TTS** to **SSML**. Plain text is shown in the speech pane but not sent for playback.

### 1. Deploy the proxy

Prerequisites for **running locally**: [AWS SAM CLI](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/install-sam-cli.html), Node.js 20+, and an AWS account.

```bash
cd lambda/tts
npm install
sam build
sam deploy --guided
```

During `sam deploy --guided`, set `TtsProvider` to `polly`, `google`, or `azure`. For Google or Azure, also supply the credential parameters (or add them later in the Lambda console). Copy the **`TtsApiUrl`** output from the deploy (it ends with `/tts`).

For **running on the web**, push changes under `lambda/tts/` to `main` and use the GitHub Actions workflow [`.github/workflows/deploy-tts-lambda.yml`](.github/workflows/deploy-tts-lambda.yml). Required repository secrets: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`. Optional: `TTS_PROVIDER`, `GOOGLE_SERVICE_ACCOUNT_JSON`, `AZURE_SPEECH_KEY`, `AZURE_SPEECH_REGION`.

CORS is preconfigured for the live demo and local dev: `https://daisy.github.io`, `http://localhost:8080`, and `http://127.0.0.1:8080` (see `AllowedOrigins` in [`lambda/tts/template.yaml`](lambda/tts/template.yaml)). Change that parameter if you host the demo elsewhere.

See [`lambda/tts/README.md`](lambda/tts/README.md) for API details, security notes, and billing alerts.

### 2. Point the demo at the proxy

In **`index.html`** (before building), set the API URL:

```html
window.MATHCAT_TTS_API = 'https://YOUR-API-ID.execute-api.REGION.amazonaws.com/tts';
```

Leave it as `''` to disable playback (the rest of the demo still works).

### 3. Build and run

Local:

```bash
trunk serve
```

Open the app, choose **SSML** under TTS, generate speech, and confirm audio plays with navigation highlighting.

For GitHub Pages, push frontend changes to `main` (site root) or `braille-parser` (`/input/`). The workflow [`.github/workflows/deploy-pages.yml`](.github/workflows/deploy-pages.yml) runs `trunk build` against the latest successful MathCAT build on the matching branch and publishes `dist/` to `gh-pages`. Set `MATHCAT_TTS_API` in `index.html` before pushing so the built site includes the proxy URL. You can also rebuild from the Actions tab without a new commit (see [Website builds](#website-builds)).

### Switching engines later

Redeploy the Lambda with a different `TtsProvider` / `TTS_PROVIDER` and update credentials. No frontend or demo UI changes are required.

## Website builds

The live demo is at [daisy.github.io/MathCATDemo](https://daisy.github.io/MathCATDemo/). GitHub Pages serves the **`gh-pages`** branch (contents of `dist/`). The braille-parser demo is at [daisy.github.io/MathCATDemo/input](https://daisy.github.io/MathCATDemo/input/).

**Automatic deploy:** Pushing to `main` or `braille-parser` triggers [`.github/workflows/deploy-pages.yml`](.github/workflows/deploy-pages.yml). The workflow checks out [daisy/MathCAT](https://github.com/daisy/MathCAT) at the latest **successful** [Rust](https://github.com/daisy/MathCAT/actions/workflows/rust.yml) run on the matching branch (`main` → MathCAT `main`, site root; `braille-parser` → MathCAT `braille-parser`, `/input/`), runs `trunk build`, and pushes `dist/` to `gh-pages`.

**Manual rebuild:** On GitHub, open **Actions → Deploy GitHub Pages → Run workflow**, then choose `main` or `braille-parser`. That republishes the demo against the latest successful MathCAT build on the matching branch, without a new demo commit. The button appears once this workflow file is on the default branch.

**TTS Lambda deploy:** [`.github/workflows/deploy-tts-lambda.yml`](.github/workflows/deploy-tts-lambda.yml) is separate — it updates only the TTS backend when `lambda/tts/**` changes.

Set `window.MATHCAT_TTS_API` in `index.html` on `main` before pushing if you want speech on the live site (see [Text-to-speech](#text-to-speech)).

### Manual deploy (optional)

If you need to publish without pushing to `main`:

1. **Stop `trunk serve`** if it is running.
2. **Build:** `trunk build` (requires a sibling `../MathCAT/` checkout locally).
3. **Publish:**

   ```bash
   git subtree push --prefix dist origin gh-pages
   ```

4. **Verify** at [daisy.github.io/MathCATDemo](https://daisy.github.io/MathCATDemo/).

For local testing, use `http://localhost:8080/MathCATDemo/` (trailing slash required) so browser requests match the Lambda CORS allowlist.
