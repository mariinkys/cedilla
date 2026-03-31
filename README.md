<div align="center">
<br>
<img src="./resources/icons/hicolor/scalable/apps/icon.svg" width="150" />
<h1 align="center">Cedilla</h1>

![Flathub Version](https://img.shields.io/flathub/v/dev.mariinkys.Cedilla)
![Flathub Downloads](https://img.shields.io/flathub/downloads/dev.mariinkys.Cedilla)
![GitHub License](https://img.shields.io/github/license/mariinkys/cedilla)
![GitHub Repo stars](https://img.shields.io/github/stars/mariinkys/cedilla)

<h3>A markdown text editor for the COSMIC™ desktop</h3>

<img src="./resources/screenshots/showcase.gif" width=750>
<img src="./resources/screenshots/main-light.png" width=350>
<img src="./resources/screenshots/main-dark.png" width=350>

<br><br>

<a href="https://flathub.org/apps/dev.mariinkys.Cedilla">
<img width='240' alt='Download on Flathub' src='https://flathub.org/api/badge?locale=en'/>
</a>
</div>

## Install

To install your COSMIC application, you will need [just](https://github.com/casey/just), if you're on Pop!\_OS, you can install it with the following command:

```sh
sudo apt install just
```

After you install it, you can run the following commands to build and install your application:

```sh
just build-release
sudo just install
```

## Typst Support

The app has some support for [Typst](https://typst.app/) inside `code` blocks. This has been added specially for math and formulas; other Typst features may not work correctly. (But you're free to try). You can add Typst using `typ` or `typst` as the languge attribut inside a `code` block.

<div align="center">
    <img src="./resources/screenshots/typst-light.png" width=750>
</div>
    
## Attribution

> "[Pop Icons](http://github.com/pop-os/icon-theme)" by [System76](http://system76.com/) is licensed under [CC-SA-4.0](http://creativecommons.org/licenses/by-sa/4.0/)

> For Markdown and HTML rendering this app uses the amazing work of [Mrmayman](https://github.com/Mrmayman) with [Frostmark](https://github.com/Mrmayman/frostmark). (Adapted to work with libcosmic and other changes/improvments by me)

## Copyright and Licensing

Copyright 2026 © Alex Marín

Released under the terms of the [GPL-3.0](./LICENSE)
