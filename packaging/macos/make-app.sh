#!/bin/sh
# Builds VLCCraft.app and a .dmg from an already-built binary.
#   make-app.sh <binary> <version> <outdir>
set -eu

bin=$1
version=$2
out=$3

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/../.." && pwd)

rm -rf build
mkdir -p build/dmg
app=build/dmg/VLCCraft.app
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

install -m 755 "$bin" "$app/Contents/MacOS/vlccraft"

# The icon set iconutil expects, made from the single 1024 master.
master="$root/assets/vlccraft-1024.png"
if [ -f "$master" ] && command -v iconutil >/dev/null 2>&1; then
    iconset=build/VLCCraft.iconset
    mkdir -p "$iconset"
    for size in 16 32 128 256 512; do
        sips -z "$size" "$size" "$master" --out "$iconset/icon_${size}x${size}.png" >/dev/null
        double=$((size * 2))
        sips -z "$double" "$double" "$master" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
    done
    iconutil -c icns "$iconset" -o "$app/Contents/Resources/vlccraft.icns"
else
    cp "$master" "$app/Contents/Resources/vlccraft.png" 2>/dev/null || true
fi

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>VLCCraft</string>
    <key>CFBundleDisplayName</key><string>VLCCraft</string>
    <key>CFBundleIdentifier</key><string>com.mjb73.vlccraft</string>
    <key>CFBundleExecutable</key><string>vlccraft</string>
    <key>CFBundleIconFile</key><string>vlccraft</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSRequiresAquaSystemAppearance</key><false/>
    <key>LSApplicationCategoryType</key><string>public.app-category.video</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key><string>Video</string>
            <key>CFBundleTypeRole</key><string>Viewer</string>
            <key>LSHandlerRank</key><string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.movie</string>
                <string>public.mpeg-4</string>
                <string>org.matroska.mkv</string>
                <string>com.apple.quicktime-movie</string>
            </array>
        </dict>
        <dict>
            <key>CFBundleTypeName</key><string>Audio</string>
            <key>CFBundleTypeRole</key><string>Viewer</string>
            <key>LSHandlerRank</key><string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.audio</string>
                <string>public.mp3</string>
                <string>public.aiff-audio</string>
                <string>org.xiph.flac</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
EOF

# Ad-hoc signature: enough for macOS to launch the bundle locally. No
# Developer ID, so the first open needs right-click → Open.
codesign --force --deep --timestamp=none -s - "$app" >/dev/null 2>&1 || true

cp "$root/README.md" build/dmg/README.md
cp "$root/LICENSE" build/dmg/LICENSE

mkdir -p "$out"
dmg="$out/VLCCraft-$version.dmg"
rm -f "$dmg"
hdiutil create -quiet -volname "VLCCraft $version" -srcfolder build/dmg -ov -format UDZO "$dmg"
echo "$dmg"
