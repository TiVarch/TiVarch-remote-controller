import 'package:flutter/material.dart';
import '../../core/theme/app_theme.dart';
import '../../models/connection_state.dart';

class StatusBar extends StatelessWidget {
  final ConnectionStatus status;

  const StatusBar({super.key, required this.status});

  @override
  Widget build(BuildContext context) {
    Color dotColor = AppTheme.danger;
    if (status.medium == TransportMedium.wifi) dotColor = AppTheme.success;
    if (status.medium == TransportMedium.ble) dotColor = AppTheme.accent;

    return Row(
      children: [
        CircleAvatar(radius: 5, backgroundColor: dotColor),
        const SizedBox(width: 8),
        Text(status.label, style: const TextStyle(fontSize: 13, color: AppTheme.muted)),
      ],
    );
  }
}