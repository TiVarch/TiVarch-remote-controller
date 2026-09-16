import 'package:flutter/material.dart';

class AppTheme {
  static const Color background = Color(0xFF0D1117);
  static const Color card = Color(0xFF161B22);
  static const Color border = Color(0xFF30363D);
  static const Color surface = Color(0xFF21262D);
  static const Color accent = Color(0xFF58A6FF);
  static const Color text = Color(0xFFF0F6FC);
  static const Color muted = Color(0xFF8B949E);
  static const Color danger = Color(0xFFF85149);
  static const Color success = Color(0xFF3FB950);

  static ThemeData get darkTheme {
    return ThemeData.dark().copyWith(
      scaffoldBackgroundColor: background,
      cardColor: card,
      colorScheme: const ColorScheme.dark(
        primary: accent,
        surface: card,
      ),
    );
  }
}