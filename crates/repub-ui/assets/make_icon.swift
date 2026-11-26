// Renders the Repub app icon (a white R on a flat teal square) to a 1024x1024 PNG.
// Usage: swift make_icon.swift <output.png>
//
// Regenerate the shipped assets (macOS):
//   swift make_icon.swift /tmp/icon_1024.png
//   mkdir AppIcon.iconset
//   for s in 16 32 128 256 512; do
//     sips -z $s $s /tmp/icon_1024.png --out AppIcon.iconset/icon_${s}x${s}.png
//     sips -z $((s*2)) $((s*2)) /tmp/icon_1024.png --out AppIcon.iconset/icon_${s}x${s}@2x.png
//   done
//   iconutil -c icns AppIcon.iconset -o AppIcon.icns
//   cp AppIcon.iconset/icon_256x256.png icon.png
//
// icon.png is embedded as the window icon (main.rs). AppIcon.icns goes in a
// macOS bundle at Contents/Resources/AppIcon.icns with
// CFBundleIconFile = AppIcon in Info.plist (cargo-bundle: `icon = ["assets/AppIcon.icns"]`).

import AppKit

let size: CGFloat = 1024
let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "icon_1024.png"

let rep = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: Int(size), pixelsHigh: Int(size),
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
let gctx = NSGraphicsContext(bitmapImageRep: rep)!
NSGraphicsContext.current = gctx
let cg = gctx.cgContext

func hex(_ v: UInt32) -> NSColor {
    NSColor(srgbRed: CGFloat((v >> 16) & 0xFF) / 255, green: CGFloat((v >> 8) & 0xFF) / 255,
            blue: CGFloat(v & 0xFF) / 255, alpha: 1)
}

// Top-left based rect helper (AppKit is bottom-left)
func rect(_ x0: CGFloat, _ y0: CGFloat, _ x1: CGFloat, _ y1: CGFloat) -> NSRect {
    NSRect(x: x0, y: size - y1, width: x1 - x0, height: y1 - y0)
}

func rounded(_ r: NSRect, _ radius: CGFloat) -> NSBezierPath {
    NSBezierPath(roundedRect: r, xRadius: radius, yRadius: radius)
}

// Flat teal rounded square with a white "R"
let tile = rect(100, 100, 924, 924)
hex(0x0E8F80).setFill()
rounded(tile, 200).fill()

let font = NSFont.systemFont(ofSize: 640, weight: .bold)
let attrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: NSColor.white]
let letter = NSAttributedString(string: "R", attributes: attrs)
// draw(at:) places the line box, whose baseline sits `descender` above its bottom edge;
// center the capital's height and advance in the tile
letter.draw(at: NSPoint(
    x: tile.midX - letter.size().width / 2,
    y: tile.midY - font.capHeight / 2 + font.descender))

gctx.flushGraphics()
let data = rep.representation(using: .png, properties: [:])!
try! data.write(to: URL(fileURLWithPath: out))
