import 'package:flutter/material.dart';
import '../../core/theme/app_theme.dart';

class KeyboardDialog extends StatelessWidget {
  final Function(String) onSubmit;
  final TextEditingController _controller = TextEditingController();

  KeyboardDialog({super.key, required this.onSubmit});

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: AppTheme.card,
      title: const Text('Text to Bigscreen'),
      content: TextField(
        controller: _controller,
        autofocus: true,
        decoration: const InputDecoration(hintText: 'Search or URL...'),
        onSubmitted: (val) {
          onSubmit(val);
          Navigator.pop(context);
        },
      ),
      actions: [
        TextButton(
          onPressed: () {
            onSubmit(_controller.text);
            Navigator.pop(context);
          },
          child: const Text('SEND'),
        ),
      ],
    );
  }
}