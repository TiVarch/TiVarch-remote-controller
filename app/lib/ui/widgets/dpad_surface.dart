import 'package:flutter/material.dart';
import '../../core/theme/app_theme.dart';

class DpadSurface extends StatelessWidget {
  final Function(String action, String state) onDpad;

  const DpadSurface({super.key, required this.onDpad});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 260,
      height: 260,
      decoration: BoxDecoration(
        color: AppTheme.card,
        shape: BoxShape.circle,
        border: Border.all(color: AppTheme.border, width: 2),
      ),
      child: Stack(
        alignment: Alignment.center,
        children: [
          Positioned(top: 8, child: _btn(Icons.arrow_drop_up, 'up')),
          Positioned(bottom: 8, child: _btn(Icons.arrow_drop_down, 'down')),
          Positioned(left: 8, child: _btn(Icons.arrow_left, 'left')),
          Positioned(right: 8, child: _btn(Icons.arrow_right, 'right')),
          GestureDetector(
            onTap: () => onDpad('select', 'click'),
            child: Container(
              width: 84,
              height: 84,
              decoration: const BoxDecoration(
                color: AppTheme.surface,
                shape: BoxShape.circle,
              ),
              child: const Center(
                child: Text('OK', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 18)),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _btn(IconData icon, String action) {
    return Listener(
      onPointerDown: (_) => onDpad(action, 'press'),
      onPointerUp: (_) => onDpad(action, 'release'),
      child: Container(
        padding: const EdgeInsets.all(12),
        color: Colors.transparent,
        child: Icon(icon, size: 40, color: AppTheme.muted),
      ),
    );
  }
}