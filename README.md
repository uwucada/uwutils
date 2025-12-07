# uwutils (´• ω •`)

a collection of small, cute ARG helper utils written in rust ✨

## what's inside 「(•ˋ _ ˊ•)」 ↵

### uwu-pdf ↵
PDF utilities for extraction and analysis

- **extract**: pull images and resources from PDF files
- **analyze**: inspect PDF structure and metadata

### uwu-qr ↵
QR code reader that works with files or clipboard

- reads QR codes from image files
- can read directly from your clipboard (no file needed!)

### uwu-atag ↵
audio tag dumper for various formats

- dumps metadata tags from audio files
- supports MP3, FLAC, and other common formats

## installation ↵

run the install script:
```bash
./install.sh
```

this will build all utilities in release mode and install them to your local bin directory, if the directory isn't on path it'll ask you to add it :3 

## building manually ↵

```bash
cargo build --release --workspace
```

binaries will be in `target/release/`

## usage examples ↵

```bash
# extract images from a PDF
uwu-pdf extract -i document.pdf -o output_folder
```

```bash
# analyze PDF structure
uwu-pdf analyze -i document.pdf
```

```bash
# read QR code from file
uwu-qr -i qrcode.png
```

```bash
# read QR code from clipboard
uwu-qr
```

```bash
# dump audio tags
uwu-atag -i song.mp3
```

## requirements ↵
- cargo

## compatibility ↵
these are all built for linux-first and should support mac 

they may work on windows but any functionality on windows is purely incidental 

we do not like windows ( ｡ •̀ ᴖ •́ ｡)💢

---

made with ♡ by team uwucada (ﾉ◕ヮ◕)ﾉ*:･ﾟ✧
