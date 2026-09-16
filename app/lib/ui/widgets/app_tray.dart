import 'package:flutter/material.dart';
import '../../core/theme/app_theme.dart';

class AppTray extends StatelessWidget {
  final Function(String) onLaunch;

  const AppTray({super.key, required this.onLaunch});

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceEvenly,
      children: [
        _appButton('YouTube', Icons.play_circle_fill, Colors.red, 'youtube'),
        _appButton('Kodi', Icons.movie_filter, Colors.lightBlue, 'kodi'),
        _appButton('Steam', Icons.sports_esports, Colors.white70, 'steam'),
      ],
    );
  }

  Widget _appButton(String title, IconData icon, Color color, String id) {
    return InkWell(
      onTap: () => onLaunch(id),
      borderRadius: BorderRadius.circular(12),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        decoration: BoxDecoration(
          color: AppTheme.card,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: AppTheme.border),
        ),
        child: Row(
          children: [
            Icon(icon, color: color, size: 20),
            const SizedBox(width: 6),
            Text(title, style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600)),
          ],
        ),
      ),
    );
  }
}