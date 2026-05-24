# MathCAT: Math Capable Assistive Technology Demo
<img src="logo.png" style="position: relative; top: 16px; z-index: -1;"> is a library that supports conversion of MathML to speech and braille among other things.
This project adds a GUI to MathCAT to demo some of its capabilities.
Visit [the MathCAT project page](https://daisy.github.io/MathCAT/) for more info or if you want to play around, [try out the demo](https://daisy.github.io/MathCATDemo/).


## Local builds
To build this and run locally, you need to download and install [trunk](https://trunkrs.dev/guide/getting-started/installation.html). Then type
```
trunk serve
```

## Website builds
To upload to the github website, do the following ([based on this github page](https://gist.github.com/cobyism/4730490)):
1. stop trunk serve (it will rebuild the file and wipe the following change)
2. In dist/index.html: update the line that starts:
```
    import {load_yaml_file} from '/MathCATDemo/index-
```
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;with the value from the line
```
    <link rel="modulepreload" href="/MathCATDemo/index-
```

```
3. In the shell, issue the commands (the first might not be needed)
```
    git push origin --delete gh-pages
    git add dist && git commit -m "update"
    c:/Software/Git/bin/git subtree push --prefix dist origin gh-pages 
```

Note: step '2' shouldn't be needed, but I haven't figured out the configuration settings to get it to do the update properly.
