// Renders every SVG in assets/illustrations to PNG with flutter_svg itself, so what
// you see is what the app draws (browsers render some SVG features that flutter_svg
// does not, e.g. <pattern>). Not part of `flutter test`; run it explicitly:
//
//   scripts/illustrations/preview.sh          (or, from app/:)
//   flutter test tool/illustrations/preview_test.dart
//
// Output (ILLUSTRATION_PREVIEW_OUT, default build/illustration_preview/):
//   <name>.png        the raw art at 1080 px wide
//   <name>_hero.png   portrait art (heroes) as OnboardingHero shows it: phone-sized
//                     frame in the theme's window colour with the same fade/scrim
//   contact_sheet.png every raw illustration side by side
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:flutter_test/flutter_test.dart';

const _windowDark = Color(0xFF0F0F0F);
const _windowLight = Color(0xFFF7F7F7);
const _width = 1080.0;
const _phone = Size(1080, 2400);

Future<void> _writePng(ui.Image image, String path) async {
  final data = await image.toByteData(format: ui.ImageByteFormat.png);
  File(path).writeAsBytesSync(data!.buffer.asUint8List());
}

ui.Image _record(Size size, void Function(Canvas) paint) {
  final recorder = ui.PictureRecorder();
  paint(Canvas(recorder));
  return recorder.endRecording().toImageSync(
    size.width.round(),
    size.height.round(),
  );
}

void main() {
  test('render illustrations', () async {
    final out = Directory(
      Platform.environment['ILLUSTRATION_PREVIEW_OUT'] ??
          'build/illustration_preview',
    )..createSync(recursive: true);
    final files =
        Directory('assets/illustrations')
            .listSync()
            .whereType<File>()
            .where((f) => f.path.endsWith('.svg'))
            .toList()
          ..sort((a, b) => a.path.compareTo(b.path));
    expect(files, isNotEmpty);

    final thumbs = <ui.Image>[];
    for (final file in files) {
      final name = file.uri.pathSegments.last.replaceAll('.svg', '');
      final dark = name.endsWith('_dark');
      final window = dark ? _windowDark : _windowLight;
      final info = await vg.loadPicture(
        SvgStringLoader(file.readAsStringSync()),
        null,
      );
      final scale = _width / info.size.width;
      final artSize = Size(_width, info.size.height * scale);

      void drawArt(Canvas c) {
        c.save();
        c.scale(scale);
        c.drawPicture(info.picture);
        c.restore();
      }

      final raw = _record(artSize, drawArt);
      await _writePng(raw, '${out.path}/$name.png');

      thumbs.add(raw);
      if (artSize.height > artSize.width) {
        // Same stops as OnboardingHero in lib/src/features/onboarding/onboarding_art.dart.
        final hero = _record(_phone, (c) {
          c.drawRect(Offset.zero & _phone, Paint()..color = window);
          drawArt(c);
          c.drawRect(
            Offset.zero & artSize,
            Paint()
              ..shader = ui.Gradient.linear(
                Offset.zero,
                Offset(0, artSize.height),
                [
                  window.withValues(alpha: 0.85),
                  window.withValues(alpha: 0),
                  window.withValues(alpha: 0),
                  window,
                ],
                [0, 0.1, 0.4, 0.74],
              ),
          );
        });
        await _writePng(hero, '${out.path}/${name}_hero.png');
      }
      info.picture.dispose();
      // ignore: avoid_print
      print('${out.path}/$name.png');
    }

    const thumbW = 360.0, gap = 12.0;
    final thumbH = thumbs
        .map((t) => thumbW * t.height / t.width)
        .reduce((a, b) => a > b ? a : b);
    final sheet = _record(
      Size(thumbs.length * (thumbW + gap) + gap, thumbH + 2 * gap),
      (c) {
        c.drawPaint(Paint()..color = const Color(0xFF888888));
        for (var i = 0; i < thumbs.length; i++) {
          c.drawImageRect(
            thumbs[i],
            Offset.zero &
                Size(thumbs[i].width.toDouble(), thumbs[i].height.toDouble()),
            Rect.fromLTWH(
              gap + i * (thumbW + gap),
              gap,
              thumbW,
              thumbW * thumbs[i].height / thumbs[i].width,
            ),
            Paint()..filterQuality = FilterQuality.medium,
          );
        }
      },
    );
    await _writePng(sheet, '${out.path}/contact_sheet.png');
  });
}
