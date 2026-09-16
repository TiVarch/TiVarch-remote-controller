import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/remote_controller.dart';
import '../widgets/app_tray.dart';
import '../widgets/dpad_surface.dart';
import '../widgets/keyboard_dialog.dart';
import '../widgets/media_bar.dart';
import '../widgets/status_bar.dart';

class RemoteScreen extends StatelessWidget {
  const RemoteScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<RemoteController>();

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        title: StatusBar(status: controller.status),
        actions: [
          IconButton(
            icon: const Icon(Icons.keyboard),
            onPressed: () => showDialog(
              context: context,
              builder: (_) => KeyboardDialog(onSubmit: controller.sendText),
            ),
          ),
          IconButton(
            icon: const Icon(Icons.power_settings_new, color: Colors.redAccent),
            onPressed: () => controller.sendPower('suspend'),
          ),
        ],
      ),
      body: SafeArea(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: [
            AppTray(onLaunch: controller.sendAppLaunch),
            DpadSurface(onDpad: controller.sendDPad),
            MediaBar(onDpad: controller.sendDPad, onMedia: controller.sendMedia),
          ],
        ),
      ),
    );
  }
}