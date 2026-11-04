# Launchers for the desktop app

Part of `install-guide`, step 7. Run from the checkout's root, after step 4
built `app/target/release/scorsese-app`. Each launcher names the binary where
it was built, so a rebuild updates what it opens. **Check first**: if the
file a section writes is already there and names this checkout, skip it.

## Linux

```sh
ROOT="$PWD"
mkdir -p ~/.local/share/applications ~/.local/share/icons/hicolor/512x512/apps
cp app/assets/icon.png ~/.local/share/icons/hicolor/512x512/apps/scorsese.png
cat > ~/.local/share/applications/scorsese.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=scorsese
Comment=Video editor
Exec="$ROOT/app/target/release/scorsese-app"
Icon=scorsese
Terminal=false
Categories=AudioVideo;Video;
DESKTOP
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

It shows up in the app menu within a few seconds; a menu that has not noticed
picks it up at the next login.

## macOS

A minimal bundle whose executable is a two-line script that starts the built
binary, with the icon made by `sips` and `iconutil` (both ship with macOS):

```sh
ROOT="$PWD"
APP=~/Applications/scorsese.app
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
ICONSET="$(mktemp -d)/scorsese.iconset"; mkdir "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size app/assets/icon.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2)); [ $double -le 512 ] &&
    sips -z $double $double app/assets/icon.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/scorsese.icns"
printf '#!/bin/sh\nexec "%s/app/target/release/scorsese-app" "$@"\n' "$ROOT" > "$APP/Contents/MacOS/scorsese"
chmod +x "$APP/Contents/MacOS/scorsese"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>scorsese</string>
  <key>CFBundleIdentifier</key><string>dev.scorsese.app</string>
  <key>CFBundleExecutable</key><string>scorsese</string>
  <key>CFBundleIconFile</key><string>scorsese</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
touch "$APP"
```

Spotlight finds it as *scorsese*. The first open may need a right-click →
**Open**, since the bundle is not signed.

## Windows (PowerShell)

The icon is converted to `.ico` with the ffmpeg from step 2, then a Start-menu
shortcut is made through `WScript.Shell`:

```powershell
$root = (Get-Location).Path
$dir = Join-Path $env:LOCALAPPDATA "scorsese"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
ffmpeg -y -loglevel error -i app\assets\icon.png -vf scale=256:256 (Join-Path $dir "scorsese.ico")
$lnk = Join-Path ([Environment]::GetFolderPath("Programs")) "scorsese.lnk"
$shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
$shortcut.TargetPath = Join-Path $root "app\target\release\scorsese-app.exe"
$shortcut.WorkingDirectory = $root
$shortcut.IconLocation = Join-Path $dir "scorsese.ico"
$shortcut.Save()
```

It shows up in the Start menu as *scorsese*.
