import 'package:flutter/material.dart';
import '../../core/constants/ble_constants.dart';
import '../../core/theme/app_theme.dart';

class MediaBar extends StatelessWidget {
  final Function(String action, String state, {int? bleMask}) onDpad;
  final Function(String action, {int? bleMask}) onMedia;

  const MediaBar({super.key, required this.onDpad, required this.onMedia});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          _btn('BACK', () => onDpad('back', 'click', bleMask: BleConstants.maskBack)),
          _btn('HOME', () => onDpad('home', 'click', bleMask: BleConstants.maskHome)),
          _icon(Icons.play_arrow, () => onMedia('play_pause', bleMask: BleConstants.maskPlayPause)),
          _icon(Icons.volume_down, () => onMedia('volume_down', bleMask: BleConstants.maskVolumeDown)),
          _icon(Icons.volume_up, () => onMedia('volume_up', bleMask: BleConstants.maskVolumeUp)),
        ],
      ),
    );
  }

  Widget _btn(String label, VoidCallback onTap) {
    return ElevatedButton(
      style: ElevatedButton.styleFrom(
        backgroundColor: AppTheme.card,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        side: const BorderSide(color: AppTheme.border),
      ),
      onPressed: onTap,
      child: Text(label),
    );
  }

  Widget _icon(IconData icon, VoidCallback onTap) {
    return IconButton(
      icon: Icon(icon, color: AppTheme.text),
      onPressed: onTap,
    );
  }
}